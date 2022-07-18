use crate::config::Config;
use crate::content::Deployer;
use crate::data::CopyMode;
use crate::fetch::Fetcher;
use crate::git::{GitCmd, GitProvider};
use crate::prompt::Prompt;
use crate::shortlink::Shortlink;
use anyhow::{Context, Result};
use std::path::Path;

#[derive(Debug)]
pub struct Opts {
    pub overwrite: bool,
    pub mode: CopyMode,
    pub is_git: bool,
    pub no_cache: bool,
    pub remote: Option<String>,
}
pub struct Runner {
    git: Box<dyn GitProvider>,
    pub show_progress: bool,
}

impl Default for Runner {
    fn default() -> Self {
        Self {
            git: Box::new(GitCmd::default()),
            show_progress: false,
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
        self.run_workflow(shortlink, dest, opts)
    }

    #[tracing::instrument(skip(self), err)]
    fn run_workflow(&self, shortlink: Option<&str>, dest: Option<&str>, opts: &Opts) -> Result<()> {
        let (mut config, _) = Config::load_or_default().context("could not load configuration")?;

        // optionally add remote and sync here if remote exists
        if let Some(remote) = opts.remote.as_ref() {
            let num = config.load_remote_source(remote.as_str())?;
            let prompt = Prompt::new(&config);
            if prompt.confirm_save_remotes(num)? {
                config.save()?;
            }
        }

        let config = config;
        let prompt = Prompt::new(&config);
        let should_confirm = shortlink.is_none() || dest.is_none();

        let (is_git, shortlink) = match shortlink {
            Some(s) => (opts.is_git, s.to_string()),
            None => {
                let project = prompt.pick_project(&opts.mode)?;
                if let Some(project) = project {
                    (
                        project.is_git.unwrap_or(false),
                        project.shortlink.to_string(),
                    )
                } else {
                    let shortlink = prompt.input_shortlink()?;
                    (opts.is_git, shortlink)
                }
            }
        };

        let dest = dest.map_or_else(
            || prompt.input_dest(opts.mode == CopyMode::Copy),
            |d| Ok(Some(d.to_string())),
        )?;

        if should_confirm && !prompt.are_you_sure(&shortlink, dest.as_deref())? {
            return Ok(());
        }

        if self.show_progress {
            prompt.say_resolving();
        }

        let sl = Shortlink::new(&config, self.git.as_ref());
        let (location, assets) = sl.resolve(&shortlink, is_git)?;

        let cached_path = Config::global_cache_folder()?;
        let fetcher = Fetcher::new(self.git.as_ref(), cached_path.as_path());
        if self.show_progress {
            prompt.say_fetching();
        }
        let (source, remove_source) = fetcher.fetch(&location, &assets, opts.no_cache)?;

        if self.show_progress {
            prompt.say_unpacking();
        }
        let deployer = Deployer::default();
        let res = deployer.deploy(
            Path::new(&source),
            dest.as_ref().map(Path::new),
            &location,
            &opts.mode,
            opts.overwrite,
            remove_source,
        )?;
        if self.show_progress {
            prompt.say_done(&res);
        }
        Ok(())
    }
}
