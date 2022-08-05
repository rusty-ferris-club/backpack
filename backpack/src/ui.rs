use crate::actions::ActionResult;
use crate::config::Config;
use crate::data::{CopyMode, Opts};
use anyhow::{anyhow, Context, Result as AnyResult};
use console::style;
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
    ) -> AnyResult<(String, Option<String>)> {
        let shortlink = if let Some(s) = shortlink {
            s.to_string()
        } else {
            let project = self.pick_project(&opts.mode)?;
            if let Some(project) = project {
                project
            } else {
                self.input_shortlink()?
            }
        };

        let dest = dest.map_or_else(
            || self.input_dest(opts.mode == CopyMode::Copy),
            |d| Ok(Some(d.to_string())),
        )?;

        Ok((shortlink, dest))
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
        match self.config.projects_for_selection(mode) {
            projects if !projects.is_empty() => {
                let options = projects
                    .iter()
                    .map(|(k, p)| {
                        format!(
                            "{} ({})",
                            k,
                            p.mode
                                .as_ref()
                                .map_or("apply+new", |m| if CopyMode::Apply.eq(m) {
                                    "apply"
                                } else {
                                    "new"
                                })
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
    pub fn input_dest(&mut self, guess: bool) -> AnyResult<Option<String>> {
        let default_dest = guess_dest()?;

        let mut b = Question::input("question").message("Destination");
        if guess {
            b = b.default(default_dest);
        }
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

    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn confirm_save_remotes(&mut self, num_remotes: usize) -> AnyResult<bool> {
        let question = Question::confirm("question")
            .message(format!(
                "Found {} remote(s). Would you like to save to your configuration?",
                num_remotes,
            ))
            .build();

        Ok(self.prompt_one(question)?.as_bool().unwrap_or(false))
    }

    /// Returns the input shortlink of this [`Prompt`].
    ///
    /// # Errors
    ///
    /// This function will return an error if interaction is killed
    pub fn are_you_sure(&mut self, shortlink: &str, dest: Option<&str>) -> AnyResult<bool> {
        let question = Question::confirm("question")
            .message(format!(
                "Generate from '{}' into '{}'?",
                shortlink,
                dest.unwrap_or("a default folder"),
            ))
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
        println!("{}", name);
    }

    pub fn say_actions(&self, number: usize) {
        if self.show_progress {
            println!("🍿 Running {} action(s):", number);
        }
    }

    pub fn say_done(&self, res: &[String], maybe_actions: Option<&Vec<ActionResult>>) {
        if self.show_progress {
            let total = res.len();
            let total_actions = maybe_actions.map_or(0, Vec::len);
            let cutoff = 10;

            if let Some(ars) = maybe_actions {
                println!();
                for ar in ars.iter() {
                    println!(" {} {}", style("🍿").green(), style(&ar.name).dim());
                }
                println!();
            };

            res.iter().enumerate().take(cutoff).for_each(|(i, s)| {
                if i == 0 {
                    println!();
                }
                println!(" {} {}", style("+").green(), style(s).dim());
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
        .map(|idx| PathBuf::from(format!("my-project{}", idx)))
        .find(|p| !p.exists())
        .ok_or_else(|| anyhow!("too many apps generated, pick a custom app name?"))?
        .display()
        .to_string())
}