use crate::config::Config;
use crate::content::Deployer;
use crate::data::CopyMode;
use crate::fetch::Fetcher;
use crate::git::{GitCmd, GitProvider};
use crate::shortlink::Shortlink;
use anyhow::Context;
use anyhow::Result;
use std::path::Path;

#[derive(Debug)]
pub struct Opts {
    pub overwrite: bool,
    pub mode: CopyMode,
    pub is_git: bool,
    pub no_cache: bool,
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
    pub fn run(&self, shortlink: &str, dest: Option<&str>, opts: &Opts) -> Result<()> {
        self.run_workflow(shortlink, dest, opts)
    }

    #[tracing::instrument(skip(self), err)]
    fn run_workflow(&self, shortlink: &str, dest: Option<&str>, opts: &Opts) -> Result<()> {
        if self.show_progress {
            println!("🔮 Resolving...");
        }
        let config = Config::load_or_default().context("could not load configuration")?;

        let sl = Shortlink::new(&config, self.git.as_ref());
        let (location, assets) = sl.resolve(shortlink, opts.is_git)?;

        let cached_path = Config::global_cache_folder()?;
        let fetcher = Fetcher::new(self.git.as_ref(), cached_path.as_path());
        if self.show_progress {
            println!("🚚 Fetching content...");
        }
        let (source, remove_source) = fetcher.fetch(&location, &assets, opts.no_cache)?;

        if self.show_progress {
            println!("🎒 Unpacking files...");
        }
        let deployer = Deployer::default();
        let res = deployer.deploy(
            Path::new(&source),
            dest.map(Path::new),
            &location,
            &opts.mode,
            opts.overwrite,
            remove_source,
        )?;
        if self.show_progress {
            println!("🎉 Done in: '{}'", res);
        }
        Ok(())
    }
}
