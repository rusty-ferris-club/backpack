use crate::config::Config;
use crate::content::{Coordinate, Deployer};
use crate::data::{CopyMode, Opts};
use crate::fetch::Fetcher;
use crate::git::{self};
use crate::shortlink::Shortlink;
use crate::ui::Prompt;
use anyhow::{Context, Result};
use interactive_actions::ActionRunner;
use requestty_ui::events::KeyEvent;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Default)]
pub struct Runner {
    git: Box<git::GitCmd>,
}

#[derive(Clone, Debug)]
pub struct RunnerEvents {
    pub prompt_events: Option<Vec<KeyEvent>>,
    pub actions_events: Option<Vec<KeyEvent>>,
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
        // if apply and no dest, it's always current folder
        let dest = dest.or_else(|| {
            if opts.mode == CopyMode::Apply {
                Some(".")
            } else {
                None
            }
        });

        // load from direct file, or magically load from 'local' then 'global', then default
        let config = opts.config_file.as_ref().map_or_else(
            || Config::load_or_default().context("could not load configuration"),
            |f| Config::from_path(Path::new(f)),
        )?;

        let config = config;

        let prompt = &mut Prompt::build(&config, opts.show_progress, events);

        let (shortlink, dest, should_confirm) = prompt.fill_missing(shortlink, dest)?;

        let sl = Shortlink::new(&config, self.git.as_ref());

        let mut vars: BTreeMap<String, String> = BTreeMap::new();

        prompt.say_resolving();
        let (location, assets) = sl.resolve(&shortlink, opts.is_git)?;

        let cached_path = Config::global_cache_folder()?;
        let fetcher = Fetcher::new(self.git.as_ref(), cached_path.as_path());

        prompt.say_fetching();
        let (source, remove_source) = fetcher.fetch(&location, &assets, opts.no_cache)?;

        let project_setup = sl.setup_actions(&shortlink);

        let mut action_runner = build_runner(events);
        let mut deployer = Deployer::new(&mut action_runner);

        let coords = Coordinate::new(
            source.as_path(),
            dest.as_deref().map(Path::new),
            &location,
            remove_source,
        )?;

        // confirm
        if !opts.always_yes
            && should_confirm
            && !prompt.are_you_sure(&format!(
                "Generate from '{}' into '{}'?",
                shortlink,
                coords.to.to_string_lossy(),
            ))?
        {
            // bail out, user won't confirm
            return Ok(());
        }

        prompt.say_unpacking();
        let (files, maybe_actions) =
            deployer.deploy(coords, project_setup, &mut vars, opts, prompt)?;

        prompt.say_done(&files, maybe_actions.as_ref());
        Ok(())
    }
}

/// build a runner with actions and if there are synthetic events, use them
pub fn build_runner(events: Option<&RunnerEvents>) -> ActionRunner {
    events
        .and_then(|evs| evs.actions_events.clone())
        .map_or_else(ActionRunner::default, ActionRunner::with_events)
}
