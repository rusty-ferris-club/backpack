use crate::data::Location;
use anyhow::{bail, Context, Result};
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

    /// Get the current repo's main remote url
    ///
    /// # Errors
    ///
    /// This function will return an error if underlying remote resolving implementation failed.
    fn get_local_url(&self) -> Result<String>;
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
        let rremote = remote.as_str();

        let process = Command::new("git")
            .arg("ls-remote")
            .arg(rremote)
            .output()
            .with_context(|| format!("cannot run git ls-remote on '{rremote}'"))?;
        if !process.status.success() {
            log::error!("git ls-remote failed. stderr output:");
            let err = String::from_utf8_lossy(&process.stderr);
            err.split('\n').for_each(|line| log::error!("> {}", line));
            anyhow::bail!(
                "git ls-remote '{}' failed with exit code {}\n---\n{}",
                rremote,
                process
                    .status
                    .code()
                    .map_or_else(|| "None".into(), |code| code.to_string()),
                err
            );
        }

        String::from_utf8_lossy(&process.stdout)
            .split('\n')
            .filter(|line| !line.is_empty())
            .map(|line| {
                let (revision, ref_) = line
                    .split_once('\t')
                    .ok_or_else(|| anyhow::format_err!("Output line contains no '\\t'"))?;
                anyhow::ensure!(
                    !ref_.contains('\t'),
                    "Output line contains more than one '\\t'"
                );
                Ok(RemoteInfo {
                    revision: revision.into(),
                    ref_: ref_.into(),
                })
            })
            .collect::<Result<Vec<RemoteInfo>>>()
    }

    #[tracing::instrument(name = "git_clone", skip_all, err)]
    fn shallow_clone(&self, location: &Location, out: &str) -> anyhow::Result<()> {
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

    fn get_local_url(&self) -> anyhow::Result<String> {
        let process = Command::new("git")
            .arg("remote")
            .arg("get-url")
            .arg("origin")
            .output()
            .with_context(|| "cannot run git remote on local repo".to_string())?;

        if !process.status.success() {
            anyhow::bail!(
                "cannot find local url: {}",
                String::from_utf8_lossy(&process.stderr[..])
            );
        }

        let url = String::from_utf8_lossy(&process.stdout[..]);
        let lines = url.lines().collect::<Vec<_>>();
        if lines.len() != 1 {
            // too many urls, cannot decide
            bail!("repo has more than one remote URL")
        }
        match lines.first() {
            Some(ln) => Ok((*ln).to_string()),
            _ => bail!("no repo remote URL found"),
        }
    }
}
