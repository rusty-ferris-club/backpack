use anyhow::Result;
use interactive_actions::data::DefaultValue;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

const RELATIVE_CAMP_FOLDER: &str = ".backpack-camp";
const RELATIVE_INTERACTION_ANSWERS_FILE: &str = "interaction-answers.yaml";

pub(crate) struct CampData {
    pub(crate) interaction_answers: HashMap<String, DefaultValue>,
}

impl CampData {
    pub fn camp_folder() -> PathBuf {
        PathBuf::from(RELATIVE_CAMP_FOLDER)
    }

    pub fn interactions_config_file() -> PathBuf {
        Self::camp_folder().join(RELATIVE_INTERACTION_ANSWERS_FILE)
    }

    pub fn interactions_answers_from_path(
        path: &Path,
    ) -> Result<Option<HashMap<String, DefaultValue>>> {
        if path.exists() {
            Ok(Some(Self::interactions_answers_from_text(
                &fs::read_to_string(path)?,
            )?))
        } else {
            Ok(None)
        }
    }

    pub fn interactions_answers_from_text(text: &str) -> Result<HashMap<String, DefaultValue>> {
        let conf: HashMap<String, DefaultValue> = serde_yaml::from_str(text)?;
        Ok(conf)
    }

    #[tracing::instrument(name = "camp_data_path", skip_all, err)]
    pub fn from_path(file: &Path) -> Result<Self> {
        Ok(Self {
            interaction_answers: Self::interactions_answers_from_path(file)?.unwrap_or_default(),
        })
    }

    #[tracing::instrument(name = "camp_data_load", skip_all, err)]
    pub fn load_or_default() -> Result<Self> {
        Self::from_path(&Self::interactions_config_file())
    }

    pub fn save_answers(&self) -> Result<()> {
        self.save_answers_to(&Self::interactions_config_file())?;
        Ok(())
    }

    pub fn save_answers_to(&self, path: &Path) -> Result<()> {
        fs::write(path, serde_yaml::to_string(&self.interaction_answers)?)?;
        Ok(())
    }
}

pub(crate) struct Camp {
    pub(crate) data: CampData,
}

impl Camp {
    pub(crate) fn new() -> Result<Self> {
        fs::create_dir_all(RELATIVE_CAMP_FOLDER)?;
        Ok(Self {
            data: CampData::load_or_default()?,
        })
    }
}
