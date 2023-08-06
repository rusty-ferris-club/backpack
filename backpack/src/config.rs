use crate::templates::Swap;
use anyhow::{anyhow, bail, Context, Result as AnyResult};
use dirs;
use interactive_actions::data::Action;
use serde_derive::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::{env, fs};

const GLOBAL_CONFIG_FOLDER: &str = ".backpack";
const GLOBAL_CONFIG_FILE: &str = "backpack.yaml";
pub const PROJECT_CONFIG_FILE: &str = ".backpack-project.yaml";
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

pub type ProjectMap = BTreeMap<String, Project>;

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(rename = "projects", default)]
    pub projects: Option<ProjectMap>,

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
        serde_yaml::to_string(self).context("cannot parse YAML")
    }

    #[tracing::instrument(name = "config_load", skip_all, err)]
    pub fn load(text: &str) -> AnyResult<Self> {
        Self::from_text(text)
    }

    #[tracing::instrument(name = "config_path", skip_all, err)]
    pub fn from_path(file: &Path) -> AnyResult<Self> {
        Self::from_text(&fs::read_to_string(file)?)
    }

    #[tracing::instrument(name = "config_load", skip_all, err)]
    pub fn load_or_default() -> AnyResult<Self> {
        Ok(Self::global_config_file()
            .and_then(|c| Self::from_path(&c))
            .unwrap_or_default())
    }

    /// Return a user's home directory
    ///
    /// # Errors
    ///
    /// This function will return an error if it cannot get the home dir
    pub fn global_config_folder() -> AnyResult<PathBuf> {
        // change config dir to be user home
        if let Ok(env_global_folder) = env::var("BP_FOLDER") {
            Ok(PathBuf::from(env_global_folder))
        } else {
            Ok(dirs::home_dir()
                .ok_or_else(|| anyhow!("no home dir found"))?
                .join(GLOBAL_CONFIG_FOLDER))
        }
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

    /// Saves this [`Config`].
    ///
    /// # Errors
    ///
    /// This function will return an error if cannot save to disk
    pub fn save(&self) -> AnyResult<()> {
        self.save_to(&Config::global_config_file()?)?;
        Ok(())
    }

    /// Save this configuration to a specified file
    ///
    /// # Errors
    ///
    /// This function will return an error if cannot save to disk
    pub fn save_to(&self, file: &Path) -> AnyResult<()> {
        fs::write(file, serde_yaml::to_string(self)?)?;
        Ok(())
    }

    fn init_to(path: &Path) -> AnyResult<()> {
        if path.exists() {
            bail!("configuration file already exists: {}", path.display());
        }
        fs::write(path, CONFIG_TEMPLATE)?;
        Ok(())
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn projects_for_selection(&self) -> Vec<(&String, &Project)> {
        self.projects
            .as_ref()
            .map(|ps| ps.iter().collect::<Vec<_>>())
            .unwrap_or_default()
    }

    pub fn project(&self, shortlink: &str) -> Option<&Project> {
        self.projects
            .as_ref()
            .and_then(|projects| projects.get(shortlink))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LocalProjectConfig {
    pub project: Project,
}
impl LocalProjectConfig {
    /// Load local project from text
    ///
    /// # Errors
    ///
    /// This function will return an error if IO fails
    pub fn from_text(text: &str) -> AnyResult<Self> {
        let conf: Self = serde_yaml::from_str(text)?;
        Ok(conf)
    }

    /// Load local project defs from path
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn from_path(path: &Path) -> AnyResult<Option<Self>> {
        if path.exists() {
            Ok(Some(Self::from_text(&fs::read_to_string(path)?)?))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Project {
    #[serde(rename = "shortlink")]
    pub shortlink: String,

    #[serde(rename = "is_git")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_git: Option<bool>,

    #[serde(rename = "description")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(rename = "actions")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<Action>>,

    #[serde(rename = "swaps")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swaps: Option<Vec<Swap>>,

    #[serde(rename = "use_camp")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_camp: Option<bool>,
}
impl Project {
    pub fn from_link(ln: &str) -> Self {
        Self {
            shortlink: ln.to_string(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSetupActions {
    #[serde(rename = "actions")]
    pub actions: Option<Vec<Action>>,

    #[serde(rename = "swaps")]
    pub swaps: Option<Vec<Swap>>,

    #[serde(rename = "use_camp")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_camp: Option<bool>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_debug_snapshot;
    use serial_test::serial;

    #[test]
    fn test_selection_filtering() {
        let config = Config::from_text(
            r###"
projects:
   copy_only: 
     shortlink: jondot/one
   apply_only: 
     shortlink: jondot/two
   all: 
     shortlink: jondot/three
"###,
        )
        .unwrap();
        assert_debug_snapshot!(config.projects_for_selection());
        assert_debug_snapshot!(config.projects_for_selection());
    }

    #[test]
    #[serial]
    fn test_merge_sources() {
        env::set_var("BP_FOLDER", ".backpack-test-ms");
        env::set_var("BP_CONF", "tests/fixtures/local-project.yaml");

        let global_folder = Config::global_config_folder().unwrap();
        assert!(global_folder.ends_with(".backpack-test-ms"));
        if global_folder.exists() {
            fs::remove_dir_all(&global_folder).unwrap();
        }
        fs::create_dir_all(&global_folder).unwrap();

        let config = Config::from_text(
            r###"
projects:
  nodejs:
    shortlink: correct/global
"###,
        )
        .unwrap();

        config
            .save_to(&global_folder.join("backpack.yaml"))
            .unwrap();

        let config_fully_loaded = Config::load_or_default().unwrap();
        assert_debug_snapshot!(config_fully_loaded);
    }

    #[test]
    #[serial]
    fn test_with_no_configs() {
        //
        // check that nothing happens when we have no configuration at all
        // no side effect happens (creating a file, or a conf folder)
        //
        env::set_var("BP_FOLDER", ".backpack-test-nc");
        env::set_var("BP_CONF", ".none.yaml");

        let config = Config::load_or_default().unwrap();
        let folder = Config::global_config_folder().unwrap();
        assert!(folder.ends_with(".backpack-test-nc"));

        if folder.exists() {
            fs::remove_dir_all(&folder).unwrap();
        }
        assert!(!folder.exists());
        assert_debug_snapshot!(config);
    }

    #[test]
    #[serial]
    fn test_with_no_global_folder() {
        env::set_var("BP_FOLDER", ".backpack-test-ngf");
        env::set_var("BP_CONF", "tests/fixtures/local-project.yaml");

        let folder = Config::global_config_folder().unwrap();
        assert!(folder.ends_with(".backpack-test-ngf"));

        if folder.exists() {
            fs::remove_dir_all(&folder).unwrap();
        }
        assert!(!folder.exists());

        let config = Config::load_or_default().unwrap();
        assert_debug_snapshot!(config);
    }
}
