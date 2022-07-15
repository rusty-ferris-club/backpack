use crate::merge;
use anyhow::{anyhow, bail, Result as AnyResult};
use dirs;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const GLOBAL_CONFIG_FOLDER: &str = ".backpack";
const GLOBAL_CONFIG_FILE: &str = "backpack.yaml";
const LOCAL_CONFIG_FILE: &str = ".backpack.yaml";
const CONFIG_TEMPLATE: &str = r###"

#
# Your backpack configuration
#
version: 1

#
# set up projects as convenient shortcuts to your starter projects or templates.
# $ backpack new rust-starter
#
# projects:
#   rust-starter: 
#     shortlink: jondot/rust-starter  # you can use any custom prefix here too
#     # is_git: true # force fetch from ssh
#

#
# set up custom vendor prefixes, for convenience and also for custom git
# URLs such as hosted github or gitlab and others.
# $ backpack new ghe:jondot/rust-starter
#
# vendors:
#   # overrides the default git vendor when you don't specify a prefix.
#   # $ backpack my-org/my-repo
#   default: 
#     kind: gitlab # options: gitlab | github | bitbucket
#     base: my.gitlab.com
#   custom:
#     # custom github org to prefix, and also overrides the 'gh:' prefix.
#     # $ backpack new gh:my-repo my-repo
#     gh:
#       kind: github
#       base: github.com/my-org
#
#     # sets the 'ghe' prefix to a custom git vendor for your organization, self-hosted.
#     # $ backpack new ghe:my-team/my-repo my-repo
#     ghe:
#       kind: github
#       base: github.enterprise.example.com
"###;

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(rename = "projects", default)]
    pub projects: Option<HashMap<String, Project>>,

    #[serde(rename = "vendors", default)]
    pub vendors: Option<VendorsConfig>,
}

impl Config {
    #[tracing::instrument(name = "config_from_text", skip_all, err)]
    pub fn from_text(text: &str) -> AnyResult<Self> {
        let conf: Self = serde_yaml::from_str(text)?;
        Ok(conf)
    }
    #[tracing::instrument(name = "config_from_text", skip_all, err)]
    pub fn to_text(&self) -> AnyResult<String> {
        Ok(serde_yaml::to_string(self)?)
    }

    /// Return a user's home directory
    ///
    /// # Errors
    ///
    /// This function will return an error if it cannot get the home dir
    pub fn global_config_folder() -> AnyResult<PathBuf> {
        // change config dir to be user home
        Ok(dirs::home_dir()
            .ok_or_else(|| anyhow!("no home dir found"))?
            .join(GLOBAL_CONFIG_FOLDER))
    }

    pub fn local_config_file() -> PathBuf {
        Path::new(LOCAL_CONFIG_FILE).to_path_buf()
    }

    /// Get the global config file location
    ///
    /// # Errors
    ///
    /// This function will return an error if there's no home directory
    pub fn global_config_file() -> AnyResult<PathBuf> {
        Self::global_config_folder().map(|c| c.join(GLOBAL_CONFIG_FILE))
    }

    /// Get the global cache location
    ///
    /// # Errors
    ///
    /// This function will return an error if there's no home directory
    pub fn global_cache_folder() -> AnyResult<PathBuf> {
        Self::global_config_folder().map(|c| c.join("cache"))
    }

    #[tracing::instrument(name = "config_load", skip_all, err)]
    pub fn load_or_default() -> AnyResult<Self> {
        // turn to vec operations
        let (local, global) = (Self::local_config_file(), Self::global_config_file()?);
        let configs = (
            if local.exists() { Some(local) } else { None },
            if global.exists() { Some(global) } else { None },
        );

        match configs {
            (None, None) => Ok(Self::default()),
            (Some(local_config), None) => Self::from_text(&fs::read_to_string(local_config)?),
            (None, Some(global_config)) => Self::from_text(&fs::read_to_string(global_config)?),
            (Some(local_config), Some(global_config)) => {
                let g = Self::from_text(&fs::read_to_string(global_config)?)?;
                let c = Self::from_text(&fs::read_to_string(local_config)?)?;
                Ok(Self::merge(g, c)?)
            }
        }
    }

    #[tracing::instrument(name = "config_merge", skip_all)]
    fn merge(base: Self, overrides: Self) -> AnyResult<Self> {
        Ok(merge::merge(&base, &overrides)?)
    }

    /// Initialize a local configuration
    ///
    /// # Errors
    ///
    /// This function will return an error if there's already a configuration file
    pub fn init_local() -> AnyResult<PathBuf> {
        let path = Self::local_config_file();
        Self::init_to(path.as_path())?;
        Ok(path)
    }

    /// Initialize a global configuration
    ///
    /// # Errors
    ///
    /// This function will return an error if it cannot create a global configuration
    pub fn init_global() -> AnyResult<PathBuf> {
        let folder = Self::global_config_folder()?;
        fs::create_dir_all(folder)?;
        let path = Self::global_config_file()?;
        Self::init_to(path.as_path())?;
        Ok(path)
    }

    fn init_to(path: &Path) -> AnyResult<()> {
        if path.exists() {
            bail!("configuration file already exists: {}", path.display());
        }
        fs::write(path, CONFIG_TEMPLATE)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    #[serde(rename = "shortlink")]
    pub shortlink: String,

    #[serde(rename = "is_git")]
    pub is_git: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VendorsConfig {
    #[serde(rename = "default")]
    pub vendors_default: Option<CustomVendor>,

    #[serde(rename = "custom")]
    pub custom: Option<HashMap<String, CustomVendor>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomVendor {
    #[serde(rename = "kind")]
    pub kind: String,

    #[serde(rename = "base")]
    pub base: String,
}
