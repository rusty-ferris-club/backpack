use crate::merge;
use anyhow::{anyhow, bail, Context, Result as AnyResult};
use dirs;
use reqwest::blocking::get;
use serde_derive::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::{env, fs};
use tracing::warn;

const GLOBAL_CONFIG_FOLDER: &str = ".backpack";
const GLOBAL_CONFIG_FILE: &str = "backpack.yaml";
const LOCAL_CONFIG_FILE: &str = ".backpack.yaml";
const CONFIG_TEMPLATE: &str = r###"

#
# Your backpack configuration
#
version: 1

project_sources:
  - name: community
    link: https://raw.githubusercontent.com/rusty-ferris-club/backpack-tap/main/main.yaml
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LoadSource {
    Default,
    Local,
    Global,
    Merged,
}

pub type ProjectMap = BTreeMap<String, Project>;

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(rename = "project_sources", default)]
    pub project_sources: Option<Vec<ProjectSource>>,

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
        Ok(serde_yaml::to_string(self)?)
    }

    #[tracing::instrument(name = "config_load", skip_all, err)]
    pub fn load(text: &str) -> AnyResult<Self> {
        let mut conf: Self = Self::from_text(text)?;
        let global_folder = Self::global_config_folder()?;
        if global_folder.exists() {
            let remote_projects = conf.load_remote_projects(&global_folder)?;
            conf.add_project_sources(&remote_projects)?;
        }
        Ok(conf)
    }

    #[tracing::instrument(name = "config_load", skip_all, err)]
    pub fn load_or_default() -> AnyResult<(Self, LoadSource)> {
        // turn to vec operations
        let (local, global) = (Self::local_config_file(), Self::global_config_file()?);
        let configs = (
            if local.exists() {
                Some(local.as_path())
            } else {
                None
            },
            if global.exists() {
                Some(global.as_path())
            } else {
                None
            },
        );

        let conf = match configs {
            (Some(local_config), None) => (
                Self::load(&fs::read_to_string(local_config)?)?,
                LoadSource::Local,
            ),
            (None, Some(global_config)) => (
                Self::load(&fs::read_to_string(global_config)?)?,
                LoadSource::Global,
            ),
            (Some(local_config), Some(global_config)) => {
                let g = Self::load(&fs::read_to_string(global_config)?)?;
                let c = Self::load(&fs::read_to_string(local_config)?)?;
                (Self::merge(g, c)?, LoadSource::Merged)
            }
            (None, None) => (Self::default(), LoadSource::Default),
        };

        Ok(conf)
    }

    /// Return a user's home directory
    ///
    /// # Errors
    ///
    /// This function will return an error if it cannot get the home dir
    pub fn global_config_folder() -> AnyResult<PathBuf> {
        // change config dir to be user home
        let env_global_folder = env::var("BP_FOLDER");
        Ok(dirs::home_dir()
            .ok_or_else(|| anyhow!("no home dir found"))?
            .join(env_global_folder.unwrap_or_else(|_| GLOBAL_CONFIG_FOLDER.to_string())))
    }

    pub fn local_config_file() -> PathBuf {
        let local_conf = env::var("BP_CONF");
        Path::new(&local_conf.unwrap_or_else(|_| LOCAL_CONFIG_FILE.to_string())).to_path_buf()
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

    /// Downloads remote external project sources that users has in their global configuration file.
    /// Each source will be saved in its own file based on the source name.
    ///
    /// # Errors
    ///
    /// This function will return an error if a network error or serialization error has occured.
    pub fn sync<F>(&self, progress: F) -> AnyResult<Vec<(String, Config)>>
    where
        F: Fn(&ProjectSource),
    {
        let mut out = vec![];
        if let Some(sources) = self.project_sources.as_ref() {
            let dest_folder = Self::global_config_folder()?;
            if !dest_folder.exists() {
                fs::create_dir_all(&dest_folder)?;
            }
            for source in sources {
                progress(source);
                let res = get(&source.link)
                    .context(format!("fetching remote source: {}", &source.link))?
                    .text()?;
                // tests deserialize, only save file if valid
                let mut conf = Config::from_text(&res)?;

                if let Some(projects) = conf.projects.as_mut() {
                    projects
                        .iter_mut()
                        .for_each(|(_, v)| v.source = ProjectSourceKind::External);
                }

                fs::write(
                    dest_folder.join(source.file_name()),
                    serde_yaml::to_string(&conf)?,
                )?;
                out.push((source.file_name(), conf));
            }
        }

        Ok(out)
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

    /// Save configuration to a file
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

    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn load_remote_projects(
        &self,
        global_folder: &Path,
    ) -> AnyResult<Vec<(String, Option<ProjectMap>)>> {
        self.project_sources.as_ref().map_or_else(|| Ok(vec![]),|sources| {
            sources
                .iter()
                .map(|source| {
                    let source_file = source.file_name();
                    let read_from = global_folder.join(&source_file);
                    if read_from.exists() {
                        Ok((source_file, Config::from_text(&fs::read_to_string(&read_from)?)?.projects))
                    } else {
                        warn!(
                            "{} does not exist for project source '{}'. please run `bp config --sync`",
                            source_file, source.name,
                        );
                        Ok((source_file, None))
                    }
                })
                .collect::<AnyResult<Vec<_>>>()
        })
    }
    /// Adds projects from external project sources that are configured and exists on disk.
    ///
    /// # Errors
    ///
    /// This function will return an error if a source is configured but was not sync'd yet
    pub fn add_project_sources(
        &mut self,
        remote_projects: &[(String, Option<ProjectMap>)],
    ) -> AnyResult<()> {
        let mut projects = self.projects.clone().unwrap_or_default();
        for (_, pmap) in remote_projects {
            if let Some(pmap) = pmap.as_ref() {
                for (k, v) in pmap.iter() {
                    if projects.get(k).is_none() {
                        projects.insert(k.clone(), v.clone());
                    }
                }
            };
        }
        self.projects = Some(projects);
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSource {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "link")]
    pub link: String,
}
impl ProjectSource {
    pub fn file_name(&self) -> String {
        format!("{}.yaml", self.name)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectSourceKind {
    #[serde(rename = "local")]
    Local,
    #[serde(rename = "external")]
    External,
}

impl Default for ProjectSourceKind {
    fn default() -> Self {
        Self::Local
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    #[serde(rename = "shortlink")]
    pub shortlink: String,

    #[serde(rename = "is_git")]
    pub is_git: Option<bool>,

    #[serde(rename = "description")]
    pub description: Option<String>,

    #[serde(rename = "run")]
    pub run: Option<String>,

    #[serde(rename = "source")]
    #[serde(default)]
    pub source: ProjectSourceKind,
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
    /*
    what happens if i have local project, and remote project overwriting it?
     */

    #[test]
    #[serial]
    fn test_remote_cannot_overwrite() {
        env::set_var("BP_FOLDER", ".backpack-test-rco");
        env::set_var("BP_CONF", "tests/fixtures/local-project.yaml");

        let global_folder = Config::global_config_folder().unwrap();
        assert!(global_folder.ends_with(".backpack-test-rco"));
        if global_folder.exists() {
            fs::remove_dir_all(&global_folder).unwrap();
        }
        fs::create_dir_all(&global_folder).unwrap();

        let config = Config::from_text(
            r###"
project_sources:
  - name: community
    link: https://raw.githubusercontent.com/rusty-ferris-club/backpack-tap/main/integration-test.yaml
"###,
        )
        .unwrap();

        config
            .save_to(&global_folder.join("backpack.yaml"))
            .unwrap();

        let config = Config::load_or_default().unwrap();
        let config = config.0;
        let res = config.sync(|_| {});
        assert_debug_snapshot!(res);

        let config_fully_loaded = Config::load_or_default().unwrap();
        assert_debug_snapshot!(config_fully_loaded);
    }

    #[test]
    #[serial]
    fn test_merge_sources() {
        env::set_var("BP_FOLDER", ".backpack-test-ms");
        env::set_var("BP_CONF", "tests/fixtures/merge-sources.yaml");

        let global_folder = Config::global_config_folder().unwrap();
        assert!(global_folder.ends_with(".backpack-test-ms"));
        if global_folder.exists() {
            fs::remove_dir_all(&global_folder).unwrap();
        }
        fs::create_dir_all(&global_folder).unwrap();

        let config = Config::from_text(
            r###"
project_sources:
  - name: community
    link: https://raw.githubusercontent.com/rusty-ferris-club/backpack-tap/main/integration-test.yaml
"###,
        )
        .unwrap();

        config
            .save_to(&global_folder.join("backpack.yaml"))
            .unwrap();

        let config = Config::load_or_default().unwrap();
        assert_debug_snapshot!(config);
        let config = config.0;
        let res = config.sync(|_| {});
        assert_debug_snapshot!(res);

        let config_fully_loaded = Config::load_or_default().unwrap();
        assert_debug_snapshot!(config_fully_loaded);
    }
    //
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
        assert!(!folder.join("community.yaml").exists());
        let config = config.0;
        let res = config.sync(|_| {});
        assert!(!folder.exists());
        assert!(!folder.join("community.yaml").exists());
        assert_debug_snapshot!(res);
    }

    #[test]
    #[serial]
    fn test_with_no_global_folder() {
        env::set_var("BP_FOLDER", ".backpack-test-ngf");
        env::remove_var("BP_CONF");

        let config = Config::from_text(
            r###"
project_sources:
  - name: community
    link: https://raw.githubusercontent.com/rusty-ferris-club/backpack-tap/main/integration-test.yaml
"###,
        )
        .unwrap();
        let folder = Config::global_config_folder().unwrap();
        assert!(folder.ends_with(".backpack-test-ngf"));

        if folder.exists() {
            fs::remove_dir_all(&folder).unwrap();
        }
        assert!(!folder.exists());
        assert!(!folder.join("community.yaml").exists());
        let res = config.sync(|_| {});
        assert!(folder.exists());
        assert!(folder.join("community.yaml").exists());
        assert!(!folder.join("backpack.yaml").exists());
        assert_debug_snapshot!(res);

        // there ARE some projects yamls and a global folder, but no local or global config file that gave the
        // instructions, so no reason to load anything.
        let empty_config = Config::load_or_default().unwrap();
        assert_debug_snapshot!(empty_config);

        // leave the yamls there, only save the in-memory config to disk which contains the remote
        // source info, so now it should load the sources data
        config.save_to(&folder.join("backpack.yaml")).unwrap();
        let filled_config = Config::load_or_default().unwrap();
        assert_debug_snapshot!(filled_config);
    }

    #[test]
    #[serial]
    fn test_remote_project_sources_fetch() {
        env::set_var("BP_FOLDER", ".backpack-test-rpsf");
        env::remove_var("BP_CONF");

        let config = Config::from_text(
            r###"
project_sources:
  - name: community
    link: https://raw.githubusercontent.com/rusty-ferris-club/backpack-tap/main/integration-test.yaml
"###,
        )
        .unwrap();
        let res = config.sync(|_| {});

        let f = Config::global_config_folder()
            .unwrap()
            .join("community.yaml");
        assert!(f.exists());
        fs::remove_file(&f).unwrap();

        assert_debug_snapshot!(res);
    }
}
