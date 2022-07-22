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
            path: path.to_string(),
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
