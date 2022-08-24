use crate::config::Config;
use crate::content::{Coordinate, Deployer};
use crate::data::Opts;
use crate::fetch::Fetcher;
use crate::git::{GitCmd, GitProvider};
use crate::shortlink::Shortlink;
use crate::ui::Prompt;
use anyhow::{Context, Result};
use interactive_actions::data::{Action, ActionHook};
use interactive_actions::ActionRunner;
use requestty_ui::events::KeyEvent;
use std::collections::BTreeMap;
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
        let prompt = &mut Prompt::build(&config, opts.show_progress, events);

        let (shortlink, dest, should_confirm) = prompt.fill_missing(shortlink, dest, opts)?;

        // confirm
        if !opts.always_yes
            && should_confirm
            && !prompt.are_you_sure(&shortlink, dest.as_deref())?
        {
            // bail out, user won't confirm
            return Ok(());
        }

        let sl = Shortlink::new(&config, self.git.as_ref());

        let actions = sl.actions(&shortlink);
        let mut action_runner = actions.map(|acts| build_runner(acts, events));

        let mut vars: BTreeMap<String, String> = BTreeMap::new();

        // run 'before' actions. they're silent and mostly for prep and input so no need
        // to print them out or print a summary
        if let Some(action_runner) = action_runner.as_mut() {
            action_runner.run(
                dest.as_ref().map(Path::new),
                &mut vars,
                ActionHook::Before,
                None::<fn(&Action)>,
            )?;
        }

        prompt.say_resolving();
        let (location, assets) = sl.resolve(&shortlink, opts.is_git)?;

        let cached_path = Config::global_cache_folder()?;
        let fetcher = Fetcher::new(self.git.as_ref(), cached_path.as_path());

        prompt.say_fetching();
        let (source, remove_source) = fetcher.fetch(&location, &assets, opts.no_cache)?;

        let deployer = Deployer::default();

        let coords = Coordinate {
            source: source.as_path(),
            dest: dest.as_ref().map(Path::new),
            location: &location,
            remove_source,
        };
        /*
        - move all actions back inside deployer
        - action_runner should get actions in `run` to decouple
        - so deploy should get: actions, swaps, rigged runner
            - rigged runner could move to deployer ctor
        - then, discovering source actions could be done here.
         */

        prompt.say_unpacking();
        let (files, maybe_actions) = deployer.deploy(
            coords,
            action_runner,
            sl.swaps(&shortlink),
            &mut vars,
            opts,
            prompt,
        )?;

        prompt.say_done(&files, maybe_actions.as_ref());
        Ok(())
    }
}

/// build a runner with actions and if there are synthetic events, use them
pub fn build_runner<'a>(acts: &'a [Action], events: Option<&RunnerEvents>) -> ActionRunner<'a> {
    events
        .and_then(|evs| evs.actions_events.clone())
        .map_or_else(
            || ActionRunner::new(acts),
            |evs| ActionRunner::with_events(acts, evs),
        )
}
