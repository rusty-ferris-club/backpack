use crate::actions::ActionRunner;
use crate::config::Config;
use crate::content::{Coordinate, Deployer};
use crate::data::Opts;
use crate::fetch::Fetcher;
use crate::git::{GitCmd, GitProvider};
use crate::shortlink::Shortlink;
use crate::ui::Prompt;
use anyhow::{Context, Result};
use requestty_ui::events::KeyEvent;
use std::path::Path;

pub struct Runner {
    git: Box<dyn GitProvider>,
}

#[derive(Clone, Debug)]
pub struct RunnerEvents {
    pub prompt_events: Option<Vec<KeyEvent>>,
    pub actions_events: Option<Vec<KeyEvent>>,
}

impl Default for Runner {
    fn default() -> Self {
        Self {
            git: Box::new(GitCmd::default()),
        }
    }
}
impl Runner {
    /// Run the workflow with progress
    ///
    /// # Errors
    ///
    /// This function will return an error if anything in the workflow failed
    pub fn run(&self, shortlink: Option<&str>, dest: Option<&str>, opts: &Opts) -> Result<()> {
        self.run_workflow(shortlink, dest, opts, None)
    }

    /// Run the workflow with progress and synthetic test events
    ///
    /// # Errors
    ///
    /// This function will return an error if anything in the workflow failed
    pub fn run_with_events(
        &self,
        shortlink: Option<&str>,
        dest: Option<&str>,
        opts: &Opts,
        events: &RunnerEvents,
    ) -> Result<()> {
        self.run_workflow(shortlink, dest, opts, Some(events))
    }

    fn run_workflow(
        &self,
        shortlink: Option<&str>,
        dest: Option<&str>,
        opts: &Opts,
        events: Option<&RunnerEvents>,
    ) -> Result<()> {
        let (mut config, _) = Config::load_or_default().context("could not load configuration")?;

        // optionally add remote and sync here if remote exists
        if let Some(remote) = opts.remote.as_ref() {
            let num = config.fetch_and_load_remote_projects(remote.as_str())?;
            let mut prompt = Prompt::new(&config, opts.show_progress);
            if prompt.confirm_save_remotes(num)? {
                config.save()?;
            }
        }

        let config = config;
        let prompt = &mut events
            .and_then(|evs| evs.prompt_events.as_ref())
            .map_or_else(
                || Prompt::new(&config, opts.show_progress),
                |evs| Prompt::with_events(&config, evs.clone()),
            );

        let should_confirm = shortlink.is_none() || dest.is_none();

        let (shortlink, dest) = prompt.fill_missing(shortlink, dest, opts)?;

        // confirm
        if !opts.always_yes
            && should_confirm
            && !prompt.are_you_sure(&shortlink, dest.as_deref())?
        {
            return Ok(());
        }

        prompt.say_resolving();
        let sl = Shortlink::new(&config, self.git.as_ref());
        let (location, assets, actions) = sl.resolve(&shortlink, opts.is_git)?;

        let cached_path = Config::global_cache_folder()?;
        let fetcher = Fetcher::new(self.git.as_ref(), cached_path.as_path());

        prompt.say_fetching();
        let (source, remove_source) = fetcher.fetch(&location, &assets, opts.no_cache)?;

        prompt.say_unpacking();
        // rig runner with or without synthetic keyboard events
        let action_runner = actions.map(|acts| {
            events
                .and_then(|evs| evs.actions_events.clone())
                .map_or_else(
                    || ActionRunner::new(acts),
                    |evs| ActionRunner::with_events(acts, evs),
                )
        });

        let deployer = Deployer::default();

        let coords = Coordinate {
            source: source.as_path(),
            dest: dest.as_ref().map(Path::new),
            location: &location,
            remove_source,
        };
        let (files, maybe_actions) = deployer.deploy(coords, action_runner, opts, prompt)?;

        prompt.say_done(&files, maybe_actions.as_ref());
        Ok(())
    }
}
