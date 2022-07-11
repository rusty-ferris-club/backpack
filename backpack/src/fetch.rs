use anyhow::Result as AnyResult;
use cached_path::{Cache, Options};
use std::path::Path;

use crate::{
    data::{Archive, ArchiveRoot, Assets, Location},
    git::GitProvider,
};

pub struct Fetcher<'a> {
    git: &'a dyn GitProvider,
    cache_path: &'a Path,
}

impl<'a> Fetcher<'a> {
    pub fn new(git: &'a dyn GitProvider, cache_path: &'a Path) -> Self {
        Fetcher { git, cache_path }
    }
    // needs location for branches with git mode?
    #[tracing::instrument(skip_all, err)]
    pub fn fetch(
        &self,
        location: &Location,
        assets: &Assets,
        no_cache: bool,
    ) -> AnyResult<(String, bool)> {
        if location.is_git {
            self.fetch_git(location)
        } else {
            let archive = assets
                .archive
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("no archive found"))?;
            self.fetch_archive(archive, no_cache)
        }
    }

    #[tracing::instrument(skip_all, err)]
    fn fetch_git(&self, location: &Location) -> AnyResult<(String, bool)> {
        let out = tempfile::tempdir()?.path().to_str().unwrap().to_string();

        self.git.shallow_clone(location, &out)?;
        Ok((out, true))
    }

    #[tracing::instrument(skip_all, err)]
    fn fetch_archive(&self, archive: &Archive, no_cache: bool) -> AnyResult<(String, bool)> {
        let cache = Cache::builder()
            // xxx replace with user-folder cache like .cargo has
            .dir(self.cache_path.to_path_buf())
            .progress_bar(None)
            .freshness_lifetime(if no_cache { 0 } else { 60 * 60 * 24 })
            .connect_timeout(std::time::Duration::from_secs(3))
            .build()
            .unwrap();

        let extracted =
            cache.cached_path_with_options(&archive.url, &Options::default().extract())?;
        let dir = match archive.root {
            ArchiveRoot::FirstFolder => std::fs::read_dir(&extracted)
                .unwrap()
                .next()
                .unwrap()?
                .path()
                .display()
                .to_string(),

            ArchiveRoot::Folder(ref s) => extracted.join(s).display().to_string(),

            ArchiveRoot::None => extracted.display().to_string(),
        };

        Ok((dir, false))
    }
}
