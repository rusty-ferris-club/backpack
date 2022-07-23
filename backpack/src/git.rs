use crate::data::Location;
use anyhow::{anyhow, Context, Result};
use git2::{Cred, Direction, Remote, RemoteCallbacks};

use std::process::Command;
use tracing;

pub trait GitProvider {
    /// Performs a shallow Git clone.
    ///
    /// # Errors
    ///
    /// This function will return an error if underlying Git provider failed.
    fn shallow_clone(&self, location: &Location, out: &str) -> Result<()>;

    /// Perform a Git ls-remote on a remote location.
    ///
    /// # Errors
    ///
    /// This function will return an error if underlying ls-remote implementation failed.
    fn ls_remote(&self, location: &Location) -> Result<Vec<RemoteInfo>>;

    /// Get a Git ref from a remote, for a given logical branch in `locaation`.
    ///
    /// # Errors
    ///
    /// This function will return an error if underlying remote resolving implementation failed.
    fn get_ref_or_default(&self, location: &Location) -> Result<RemoteInfo>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RemoteInfo {
    pub revision: String,
    pub ref_: String,
}

#[derive(Default)]
pub struct GitCmd {}
impl GitProvider for GitCmd {
    #[tracing::instrument(name = "git_get_ref", skip_all, err)]
    fn get_ref_or_default(&self, location: &Location) -> Result<RemoteInfo> {
        let refs = self.ls_remote(location)?;
        if let Some(ref gref) = location.gref {
            let ref_ = refs
                .iter()
                .find(|r| r.ref_.ends_with(gref))
                .ok_or_else(|| anyhow::anyhow!("no such ref found: {}", gref))?;
            return Ok(ref_.clone());
        }

        let head = refs
            .iter()
            .find(|r| r.ref_ == "HEAD")
            .ok_or_else(|| anyhow::anyhow!("no HEAD ref found"))?;
        let default_branch = refs
            .iter()
            .find(|r| r.ref_ != "HEAD" && r.revision == head.revision)
            .ok_or_else(|| anyhow::anyhow!("no default branch found"))?;
        Ok(default_branch.clone())
    }

    #[tracing::instrument(name = "git_ls_remote", skip_all, err)]
    fn ls_remote(&self, location: &Location) -> Result<Vec<RemoteInfo>> {
        let remote = if location.is_git {
            location.git_url()
        } else {
            location.web_url()
        };
        let mut callbacks = RemoteCallbacks::new();
        let home = dirs::home_dir().ok_or_else(|| anyhow!("no home dir found"))?;
        let key = vec![".ssh/id_rsa", ".ssh/id_ed25519"]
            .into_iter()
            .map(|k| home.join(k))
            .find(|p| p.exists())
            .ok_or_else(|| anyhow!("no git authentication detail found"))?;

        callbacks.credentials(move |_url, username_from_url, _allowed_types| {
            Cred::ssh_key(username_from_url.unwrap(), None, key.as_path(), None)
        });
        let mut r = Remote::create_detached(remote.as_str())?;
        let connection = r.connect_auth(Direction::Fetch, Some(callbacks), None)?;
        let info = connection
            .list()?
            .iter()
            .map(|c| RemoteInfo {
                revision: c.oid().to_string(),
                ref_: c.name().to_string(),
            })
            .collect::<Vec<_>>();
        Ok(info)
    }

    #[tracing::instrument(name = "git_clone", skip_all, err)]
    fn shallow_clone(&self, location: &Location, out: &str) -> anyhow::Result<()> {
        // libgit2 has no shallow clone(!)

        // -b branch
        // need to take location, and ask it for the git url
        let giturl = location.git_url();
        let branch = location.gref.as_deref();
        let mut args = vec!["clone", "--depth=1"];
        if let Some(branch) = branch {
            args.push("-b");
            args.push(branch);
        }
        args.push(&giturl);
        args.push(out);

        let output = Command::new("git")
            .args(&args)
            .output()
            .context("git handling failed")?;
        if !output.status.success() {
            anyhow::bail!(
                "cannot clone: {}\n---\n{}",
                giturl,
                String::from_utf8_lossy(&output.stderr[..])
            );
        }
        Ok(())
    }
}
