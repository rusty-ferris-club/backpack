use crate::config::{Config, Project};
use crate::data::CopyMode;
use crate::run::Opts;
use anyhow::{anyhow, Result as AnyResult};
use console::style;
use dialoguer::theme::{ColorfulTheme, Theme};
use dialoguer::{Confirm, FuzzySelect, Input};
use std::path::PathBuf;

pub struct Prompt<'a> {
    config: &'a Config,
    theme: Box<dyn Theme>,
}

impl<'a> Prompt<'a> {
    pub fn new(config: &'a Config) -> Self {
        Self {
            config,
            theme: Box::new(ColorfulTheme::default()),
        }
    }

    /// Fill in missing arguments with a wizard
    ///
    /// # Errors
    ///
    /// This function will return an error if prompting fails
    pub fn fill_missing(
        &self,
        shortlink: Option<&str>,
        dest: Option<&str>,
        opts: &Opts,
    ) -> AnyResult<(bool, String, Option<String>)> {
        let (is_git, shortlink) = match shortlink {
            Some(s) => (opts.is_git, s.to_string()),
            None => {
                let project = self.pick_project(&opts.mode)?;
                if let Some(project) = project {
                    (
                        project.is_git.unwrap_or(false),
                        project.shortlink.to_string(),
                    )
                } else {
                    let shortlink = self.input_shortlink()?;
                    (opts.is_git, shortlink)
                }
            }
        };
        let dest = dest.map_or_else(
            || {
                if opts.no_dest_input {
                    Ok(None)
                } else {
                    self.input_dest(opts.mode == CopyMode::Copy)
                }
            },
            |d| Ok(Some(d.to_string())),
        )?;

        Ok((is_git, shortlink, dest))
    }

    /// Returns the pick project of this [`Prompt`].
    ///
    /// # Errors
    ///
    /// This function will return an error if interaction is killed
    pub fn pick_project(&self, mode: &CopyMode) -> AnyResult<Option<&Project>> {
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
                            p.mode.as_ref().map_or("all", |m| if CopyMode::Apply.eq(m) {
                                "apply"
                            } else {
                                "new"
                            })
                        )
                    })
                    .collect::<Vec<_>>();
                let selection = FuzzySelect::with_theme(self.theme.as_ref())
                    .with_prompt("📦 Project (esc for shortlink)")
                    .default(0)
                    .items(&options)
                    .interact_opt()?;
                match selection {
                    Some(s) if s < options.len() => Ok(projects.get(s).map(|(_, p)| *p)),
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
    pub fn input_shortlink(&self) -> AnyResult<String> {
        let shortlink = Input::with_theme(self.theme.as_ref())
            .allow_empty(false)
            .with_prompt("🔗 Shortlink")
            .interact()?;
        Ok(shortlink)
    }

    /// Returns the input shortlink of this [`Prompt`].
    ///
    /// # Errors
    ///
    /// This function will return an error if interaction is killed
    pub fn input_dest(&self, guess: bool) -> AnyResult<Option<String>> {
        let default_dest = guess_dest()?;
        let mut p = Input::with_theme(self.theme.as_ref());
        p.allow_empty(true).with_prompt("📂 Destination");
        if guess {
            p.default(default_dest);
        }
        match p.interact()? {
            s if s.is_empty() => Ok(None),
            s => Ok(Some(s)),
        }
    }

    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn confirm_save_remotes(&self, num_remotes: usize) -> AnyResult<bool> {
        Ok(Confirm::with_theme(self.theme.as_ref())
            .with_prompt(format!(
                "✨ Found {}{}",
                style(num_remotes).yellow(),
                style(" remote(s). Would you like to save to your configuration?").bold(),
            ))
            .default(false)
            .interact()?)
    }

    /// Returns the input shortlink of this [`Prompt`].
    ///
    /// # Errors
    ///
    /// This function will return an error if interaction is killed
    pub fn are_you_sure(&self, shortlink: &str, dest: Option<&str>) -> AnyResult<bool> {
        Ok(Confirm::with_theme(self.theme.as_ref())
            .with_prompt(format!(
                "🕺 Generate from {} {} {}{}",
                style(shortlink).yellow(),
                style("into").bold(),
                style(dest.unwrap_or("a default folder")).bold().yellow(),
                style("?").bold(),
            ))
            .default(true)
            .interact()?)
    }

    pub fn say_resolving(&self) {
        println!("🔮 Resolving...");
    }

    pub fn say_fetching(&self) {
        println!("🚚 Fetching content...");
    }

    pub fn say_unpacking(&self) {
        println!("🎒 Unpacking files...");
    }
    pub fn say_done(&self, res: &str) {
        println!("🎉 Done in: '{}'", res);
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
