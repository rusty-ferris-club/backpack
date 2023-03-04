use crate::config::Config;
use crate::data::{CopyMode, Opts};
use crate::run::RunnerEvents;
use crate::templates::CopyResult;
use anyhow::{anyhow, Context, Result as AnyResult};
use console::style;
use interactive_actions::data::ActionResult;
use requestty::{Answer, Answers, OnEsc, Question};
use requestty_ui::backend::{Size, TestBackend};
use requestty_ui::events::{KeyEvent, TestEvents};
use std::path::Path;
use std::path::PathBuf;
use std::vec::IntoIter;

pub struct Prompt<'a> {
    config: &'a Config,
    events: Option<TestEvents<IntoIter<KeyEvent>>>,
    show_progress: bool,
}

impl<'a> Prompt<'a> {
    pub fn build(config: &'a Config, show_progress: bool, events: Option<&RunnerEvents>) -> Self {
        events
            .and_then(|evs| evs.prompt_events.as_ref())
            .map_or_else(
                || Prompt::new(config, show_progress),
                |evs| Prompt::with_events(config, evs.clone()),
            )
    }

    pub fn with_events(config: &'a Config, events: Vec<KeyEvent>) -> Self {
        Self {
            config,
            events: Some(TestEvents::new(events)),
            show_progress: false,
        }
    }

    pub fn new(config: &'a Config, show_progress: bool) -> Self {
        Self {
            config,
            events: None::<TestEvents<IntoIter<KeyEvent>>>,
            show_progress,
        }
    }

    /// Fill in missing arguments with a wizard
    ///
    /// # Errors
    ///
    /// This function will return an error if prompting fails
    pub fn fill_missing(
        &mut self,
        shortlink: Option<&str>,
        dest: Option<&str>,
        opts: &Opts,
    ) -> AnyResult<(String, Option<String>, bool)> {
        match (shortlink, dest) {
            (Some(s), Some(d)) => Ok((s.to_string(), Some(d.to_string()), false)),
            (None, d) => {
                let shortlink = if let Some(project) = self.pick_project(&opts.mode)? {
                    project
                } else {
                    self.input_shortlink()?
                };
                if let Some(d) = d {
                    Ok((shortlink, Some(d.to_string()), true))
                } else {
                    Ok((shortlink, self.input_dest()?, true))
                }
            }
            (Some(s), None) => Ok((s.to_string(), self.input_dest()?, true)),
        }
    }

    /// Returns the pick project of this [`Prompt`].
    ///
    /// # Errors
    ///
    /// This function will return an error if interaction is killed
    pub fn pick_project(&self, mode: &CopyMode) -> AnyResult<Option<String>> {
        // accept free input, user wants to input a shortlink directly
        // display where each project comes from
        // impl pick dest, where we do a "my-project" and 1,2,3,4 if exists.
        // add a final confirmation with the data
        // move all UI stuff into prompt
        match self.config.projects_for_selection(Some(mode.clone())) {
            projects if !projects.is_empty() => {
                let options = projects
                    .iter()
                    .map(|(k, p)| {
                        format!(
                            "{} ({})",
                            k,
                            if CopyMode::Apply == p.mode {
                                "apply"
                            } else {
                                "new"
                            }
                        )
                    })
                    .collect::<Vec<_>>();

                let question = Question::select("question")
                    .message("Project (esc for shortlink)")
                    .on_esc(OnEsc::SkipQuestion)
                    .choices(&options)
                    .build();

                let selection = requestty::prompt(vec![question])?
                    .get("question")
                    .and_then(|a| a.as_list_item().cloned());

                match selection {
                    Some(s) if s.index < options.len() => {
                        Ok(projects.get(s.index).map(|(k, _)| (*k).to_string()))
                    }
                    _ => Ok(None),
                }
            }
            _ => Ok(None),
        }
    }

    /// Shows the list of projects
    pub fn show_projects(&self, mode: Option<CopyMode>) {
        println!("Current projects:");
        match self.config.projects_for_selection(mode) {
            projects if !projects.is_empty() => {
                for (name, project) in projects {
                    println!(
                        "- {} ({})",
                        style(name).yellow(),
                        style(&project.shortlink).dim()
                    );
                }
            }
            _ => {
                println!("You have no projects yet.");
            }
        };
    }

    /// Returns the input shortlink of this [`Prompt`].
    ///
    /// # Errors
    ///
    /// This function will return an error if interaction is killed
    pub fn ask_for_project_name(&mut self, repo: &str) -> AnyResult<String> {
        let question = Question::input("question")
            .validate(|v, _| {
                if v.is_empty() {
                    Err("cannot be empty".into())
                } else {
                    Ok(())
                }
            })
            .message(format!("A name for '{repo}'?"))
            .build();
        let name = self
            .prompt_one(question)?
            .as_string()
            .ok_or_else(|| anyhow::anyhow!("cannot parse input"))?
            .to_string();
        Ok(name)
    }

    /// Returns the input shortlink of this [`Prompt`].
    ///
    /// # Errors
    ///
    /// This function will return an error if interaction is killed
    pub fn input_shortlink(&mut self) -> AnyResult<String> {
        let question = Question::input("question")
            .validate(|v, _| {
                if v.is_empty() {
                    Err("cannot be empty".into())
                } else {
                    Ok(())
                }
            })
            .message("Shortlink")
            .build();
        let shortlink = self
            .prompt_one(question)?
            .as_string()
            .ok_or_else(|| anyhow::anyhow!("cannot parse input"))?
            .to_string();
        Ok(shortlink)
    }

    /// Returns the input shortlink of this [`Prompt`].
    ///
    /// # Errors
    ///
    /// This function will return an error if interaction is killed
    pub fn input_dest(&mut self) -> AnyResult<Option<String>> {
        let b = Question::input("question").message("Destination");
        let question = b.build();

        let dest = self
            .prompt_one(question)?
            .as_string()
            .ok_or_else(|| anyhow::anyhow!("cannot parse input"))?
            .to_string();

        match dest {
            s if s.is_empty() => Ok(None),
            s => Ok(Some(s)),
        }
    }

    /// Returns the input shortlink of this [`Prompt`].
    ///
    /// # Errors
    ///
    /// This function will return an error if interaction is killed
    pub fn are_you_sure(&mut self, text: &str) -> AnyResult<bool> {
        let question = Question::confirm("question")
            .message(text)
            .default(true)
            .build();

        Ok(self.prompt_one(question)?.as_bool().unwrap_or(false))
    }

    /// Confirm file overwrite
    ///
    /// # Errors
    ///
    /// This function will return an error if prompting does not work
    pub fn confirm_overwrite(&mut self, file: &Path) -> AnyResult<bool> {
        let question = Question::confirm("question")
            .message(format!("'{}' already exists. overwrite?", file.display()))
            .build();

        Ok(self.prompt_one(question)?.as_bool().unwrap_or(false))
    }

    pub fn say_resolving(&self) {
        if self.show_progress {
            println!("🔮 Resolving...");
        }
    }

    pub fn say_fetching(&self) {
        if self.show_progress {
            println!("🚚 Fetching content...");
        }
    }

    pub fn say_unpacking(&self) {
        if self.show_progress {
            println!("🎒 Unpacking files...");
        }
    }

    pub fn say_action(&self, name: &str) {
        println!("{name}");
    }

    pub fn say_actions(&self, number: usize) {
        if self.show_progress {
            println!("🍿 Running {number} action(s):");
        }
    }

    pub fn say_done(&self, res: &[CopyResult], maybe_actions: Option<&Vec<ActionResult>>) {
        if self.show_progress {
            let total = res.len();
            let total_actions = maybe_actions.map_or(0, Vec::len);
            let cutoff = 10;

            if let Some(ars) = maybe_actions {
                println!();
                for ar in ars.iter() {
                    println!(" {} {}", style("✅").green(), style(&ar.name).dim());
                }
                println!();
            };

            res.iter().enumerate().take(cutoff).for_each(|(i, s)| {
                if i == 0 {
                    println!();
                }
                println!(
                    " {} {} {}",
                    style("+").green(),
                    style(s.dest.display()).dim(),
                    style(format!("{:?}", s.op)).yellow().dim()
                );
            });
            if total > cutoff {
                println!(
                    "   {} {} {}",
                    style("... and").dim(),
                    style(total - cutoff).yellow(),
                    style("more file(s).").dim()
                );
            }
            println!(
                "\n🎉 Done: {} file(s) copied with {} action(s).",
                style(total).yellow(),
                style(total_actions).yellow()
            );
        }
    }

    pub fn say(&self, text: &str) {
        println!("{text}");
    }

    /// Ask if user wants to edit a file and open editor
    ///
    /// # Errors
    ///
    /// This function will return an error if IO failed
    pub fn suggest_edit(&mut self, text: &str, path: &Path) -> AnyResult<()> {
        if self.are_you_sure(text)? {
            edit::edit_file(path)?;
        }
        Ok(())
    }

    fn prompt_one<I: Into<Question<'a>>>(&mut self, question: I) -> AnyResult<Answer> {
        match self.events {
            Some(ref mut events) => {
                let mut backend = TestBackend::new(Size::from((50, 20)));
                requestty::prompt_one_with(question, &mut backend, events)
                    .context("cannot take input")
            }
            None => requestty::prompt_one(question).context("cannot take input"),
        }
    }

    /// Prompt questions
    ///
    /// # Errors
    ///
    /// This function will return an error if prompting fails
    pub fn prompt<Q>(&mut self, questions: Q) -> AnyResult<Answers>
    where
        Q: IntoIterator<Item = Question<'a>>,
    {
        match self.events {
            Some(ref mut events) => {
                let mut backend = TestBackend::new(Size::from((50, 20)));
                requestty::prompt_with(questions, &mut backend, events).context("cannot take input")
            }
            None => requestty::prompt(questions).context("cannot take input"),
        }
    }
}

/// Guess a destination name by generating a "my-projectN..99" for the user.
///
/// # Errors
///
/// This function will return an error if too many projects have been tried
pub fn guess_dest() -> AnyResult<String> {
    Ok((1..99)
        .map(|idx| PathBuf::from(format!("my-project{idx}")))
        .find(|p| !p.exists())
        .ok_or_else(|| anyhow!("too many apps generated, pick a custom app name?"))?
        .display()
        .to_string())
}
