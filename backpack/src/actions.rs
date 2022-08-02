use anyhow::{Error, Result};
use requestty::{Answer, Question};
use requestty_ui::{
    backend::{Size, TestBackend},
    events::{KeyEvent, TestEvents},
};
use run_script::IoOptions;
use serde_derive::{Deserialize, Serialize};
use std::vec::IntoIter;
use std::{collections::BTreeMap, path::Path};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Action {
    pub name: String,
    #[serde(default)]
    interaction: Option<Interaction>,
    run: Option<String>,

    #[serde(default)]
    pub ignore_exit: bool,

    #[serde(default)]
    pub break_if_cancel: bool,

    #[serde(default)]
    pub capture: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionResult {
    pub name: String,
    pub run: Option<RunResult>,
    pub response: Response,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunResult {
    pub script: String,
    pub code: i32,
    pub out: String,
    pub err: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InteractionKind {
    #[serde(rename = "confirm")]
    Confirm,
    #[serde(rename = "input")]
    Input,
    #[serde(rename = "select")]
    Select,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum Response {
    Text(String),
    Cancel,
    None,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Interaction {
    pub kind: InteractionKind,
    pub prompt: String,
    pub out: Option<String>,

    // just for kind=select
    pub options: Option<Vec<String>>,
}

impl Interaction {
    fn update_varbag(&self, input: &str, varbag: Option<&mut VarBag>) {
        varbag.map(|bag| {
            self.out
                .as_ref()
                .map(|out| bag.insert(out.to_string(), input.to_string()))
        });
    }

    /// Play an interaction
    ///
    /// # Errors
    ///
    /// This function will return an error if text input failed
    pub fn play(
        &self,
        varbag: Option<&mut VarBag>,
        events: Option<&mut TestEvents<IntoIter<KeyEvent>>>,
    ) -> Result<Response> {
        let question = self.to_question();
        let answer = if let Some(events) = events {
            let mut backend = TestBackend::new(Size::from((50, 20)));
            requestty::prompt_one_with(question, &mut backend, events)
        } else {
            requestty::prompt_one(question)
        }?;

        Ok(match answer {
            Answer::String(input) => {
                self.update_varbag(&input, varbag);

                Response::Text(input)
            }
            Answer::ListItem(selected) => {
                self.update_varbag(&selected.text, varbag);
                Response::Text(selected.text)
            }
            Answer::Bool(confirmed) if confirmed => {
                let as_string = "true".to_string();
                self.update_varbag(&as_string, varbag);
                Response::Text(as_string)
            }
            _ => {
                Response::Cancel
                // not supported question types
            }
        })
    }

    pub fn to_question(&self) -> Question<'_> {
        match self.kind {
            InteractionKind::Input => Question::input("question")
                .message(self.prompt.clone())
                .build(),
            InteractionKind::Select => Question::select("question")
                .message(self.prompt.clone())
                .choices(self.options.clone().unwrap_or_default())
                .build(),
            InteractionKind::Confirm => Question::confirm("question")
                .message(self.prompt.clone())
                .build(),
        }
    }
}

type VarBag = BTreeMap<String, String>;
pub struct ActionRunner<'a> {
    pub actions: &'a [Action],
    pub varbag: VarBag,
    pub events: Option<TestEvents<IntoIter<KeyEvent>>>,
}

impl<'a> ActionRunner<'a> {
    pub fn new(actions: &'a [Action]) -> Self {
        Self {
            varbag: VarBag::new(),
            actions,
            events: None::<TestEvents<IntoIter<KeyEvent>>>,
        }
    }
}

impl<'a> ActionRunner<'a> {
    pub fn with_events(actions: &'a [Action], events: Vec<KeyEvent>) -> Self {
        Self {
            varbag: VarBag::new(),
            actions,
            events: Some(TestEvents::new(events)),
        }
    }

    /// Runs actions
    ///
    /// # Errors
    ///
    /// This function will return an error when actions fail
    pub fn run<P>(
        &mut self,
        working_dir: Option<&Path>,
        progress: Option<&P>,
    ) -> Result<Vec<ActionResult>>
    where
        P: Fn(&Action),
    {
        self.actions
            .iter()
            .map(|action| {
                // get interactive response from the user if any is defined
                if let Some(progress) = progress {
                    progress(action);
                }

                let response = action
                    .interaction
                    .as_ref()
                    .map_or(Ok(Response::None), |interaction| {
                        interaction.play(Some(&mut self.varbag), self.events.as_mut())
                    });

                // with the defined run script and user response, perform an action
                response.and_then(|r| match (r, action.run.as_ref()) {
                    (Response::Cancel, _) => {
                        if action.break_if_cancel {
                            Err(anyhow::anyhow!("stop requested (break_if_cancel)"))
                        } else {
                            Ok(ActionResult {
                                name: action.name.clone(),
                                run: None,
                                response: Response::Cancel,
                            })
                        }
                    }
                    (resp, None) => Ok(ActionResult {
                        name: action.name.clone(),
                        run: None,
                        response: resp,
                    }),
                    (resp, Some(run)) => {
                        let mut options = run_script::ScriptOptions::new();
                        options.working_directory = working_dir.map(std::path::Path::to_path_buf);
                        options.output_redirection = if action.capture {
                            IoOptions::Pipe
                        } else {
                            IoOptions::Inherit
                        };
                        options.print_commands = true;
                        let args = vec![];

                        // varbag replacements: {{interaction.outvar}} -> value
                        let script = self.varbag.iter().fold(run.clone(), |acc, (k, v)| {
                            acc.replace(&format!("{{{{{}}}}}", k), v)
                        });

                        run_script::run(script.as_str(), &args, &options)
                            .map_err(Error::msg)
                            .and_then(|tup| {
                                if !action.ignore_exit && tup.0 != 0 {
                                    anyhow::bail!(
                                        "in action '{}': command returned exit code '{}'",
                                        action.name,
                                        tup.0
                                    )
                                }
                                Ok(tup)
                            })
                            .map(|(code, out, err)| ActionResult {
                                name: action.name.clone(),
                                run: Some(RunResult {
                                    script,
                                    code,
                                    out,
                                    err,
                                }),
                                response: resp,
                            })
                    }
                })
            })
            .collect::<Result<Vec<_>>>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_debug_snapshot;
    use requestty_ui::events::KeyCode;

    #[test]
    fn test_interaction() {
        let actions_defs: Vec<Action> = serde_yaml::from_str(
            r#"
- name: confirm-action
  interaction:
    kind: confirm
    prompt: are you sure?
    out: confirm
- name: input-action
  interaction:
    kind: input
    prompt: which city?
    default: dallas
    out: city
- name: select-action
  interaction:
    kind: select
    prompt: select transport
    options:
    - bus
    - train
    - walk
    default: bus
"#,
        )
        .unwrap();
        let events = vec![
            KeyCode::Char('y').into(), // confirm: y
            KeyCode::Enter.into(),     //
            KeyCode::Char('t').into(), // city: 'tlv'
            KeyCode::Char('l').into(), //
            KeyCode::Char('v').into(), //
            KeyCode::Enter.into(),     //
            KeyCode::Down.into(),      // select: train
            KeyCode::Enter.into(),     //
        ];
        let mut actions = ActionRunner::with_events(&actions_defs, events);
        assert_debug_snapshot!(actions
            .run(Some(Path::new(".")), None::<&fn(&Action) -> ()>)
            .unwrap());
        assert_debug_snapshot!(actions.varbag);
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_run_script() {
        let actions_defs: Vec<Action> = serde_yaml::from_str(
            r#"
    - name: input-action
      interaction:
        kind: input
        prompt: which city?
        default: dallas
        out: city
      run: echo {{city}}
      capture: true
    "#,
        )
        .unwrap();
        let events = vec![
            KeyCode::Char('t').into(), // city: 'tlv'
            KeyCode::Char('l').into(), //
            KeyCode::Char('v').into(), //
            KeyCode::Enter.into(),     //
        ];
        let mut actions = ActionRunner::with_events(&actions_defs, events);

        insta::assert_yaml_snapshot!(actions
            .run(Some(Path::new(".")), None::<&fn(&Action) -> ()>)
            .unwrap(),  {
            "[0].run.err" => ""
        });

        assert_debug_snapshot!(actions.varbag);
    }
}
