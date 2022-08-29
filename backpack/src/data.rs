use anyhow::Result as AnyResult;
use core::fmt::Debug;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Debug)]
pub struct Location {
    pub url: String,
    pub domain: String,
    pub path: String,
    pub project: String,
    pub port: Option<u16>,
    pub query: Option<String>,
    pub subfolder: Option<String>,
    pub gref: Option<String>,
    pub username: String,
    pub password: Option<String>,
    pub is_git: bool,
}
impl Location {
    pub fn git_url(&self) -> String {
        format!(
            "git@{}:{}.git",
            self.domain,
            self.path.trim_start_matches('/')
        ) // needs: auth, port
    }

    pub fn web_url(&self) -> String {
        format!("https://{}{}", self.domain, self.path) // needs: auth, port
    }

    /// Create a logical `Location` from a `Url`.
    ///
    /// # Errors
    ///
    /// This function will return an error if `url` is illegal
    pub fn from(url: &Url, is_git: bool) -> AnyResult<Self> {
        let path = url.path();
        let parts = path.split("/-/").collect::<Vec<_>>();
        let (path, subfolder) = if parts.len() == 2 {
            (parts[0], Some(parts[1].to_string()))
        } else {
            (path, None)
        };

        let project = path
            .split('/')
            .last()
            .ok_or_else(|| anyhow::anyhow!("cannot find project name"))?;

        Ok(Self {
            url: url.to_string(),
            domain: url.domain().unwrap_or_default().to_string(),
            path: path.trim_end_matches(".git").to_string(),
            project: project.to_string(),
            port: url.port(),
            query: url.query().map(str::to_string),
            subfolder,
            gref: url.fragment().map(str::to_string),
            username: url.username().to_string(),
            password: url.password().map(str::to_string),
            is_git,
        })
    }
}

#[derive(Debug)]
pub struct Archive {
    pub url: String,
    pub root: ArchiveRoot,
}

#[derive(Debug)]
pub enum ArchiveRoot {
    Folder(String),
    FirstFolder,
    None,
}
#[derive(Debug)]
pub struct Assets {
    pub archive: Option<Archive>,
    pub git: Option<String>,
}

pub enum Overwrite {
    Ask,
    Always,
    Never,
    Custom(Box<dyn Fn(&str) -> bool>),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum CopyMode {
    #[serde(rename = "new")]
    Copy,
    #[serde(rename = "apply")]
    Apply,
    #[serde(rename = "all")]
    All,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug)]
pub struct Opts {
    pub show_progress: bool,
    pub overwrite: bool,
    pub mode: CopyMode,
    pub is_git: bool,
    pub no_cache: bool,
    pub always_yes: bool,
    pub remote: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_location_urls() {
        let loc = Location::from(
            &Url::parse("https://github.com/user/repo.git").unwrap(),
            true,
        )
        .unwrap();
        assert_eq!(loc.git_url(), "git@github.com:user/repo.git");
        assert_eq!(loc.web_url(), "https://github.com/user/repo");

        let loc =
            Location::from(&Url::parse("https://github.com/user/repo").unwrap(), false).unwrap();
        assert_eq!(loc.git_url(), "git@github.com:user/repo.git");
        assert_eq!(loc.web_url(), "https://github.com/user/repo");

        let loc = Location::from(
            &Url::parse("https://github.com/user/repo.git").unwrap(),
            false,
        )
        .unwrap();
        assert_eq!(loc.git_url(), "git@github.com:user/repo.git");
        assert_eq!(loc.web_url(), "https://github.com/user/repo");
    }
}
