use crate::data::CopyMode;
use crate::data::Location;
use crate::data::Overwrite;
use anyhow::Result;
use dialoguer;
use dialoguer::theme::ColorfulTheme;
use std::fs;
use std::path::Path;
use walkdir;

#[derive(Default)]
pub struct Deployer {}

#[tracing::instrument(skip_all, err)]
fn copy_dir(source: &Path, dest: &Path, overwrite: Overwrite) -> Result<()> {
    walkdir::WalkDir::new(source)
        .into_iter()
        .try_for_each(|entry| {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(parent) = path.parent() {
                    let dest_parent = dest.join(parent.strip_prefix(source)?);
                    if !dest_parent.exists() {
                        // Create the same dir concurrently is ok according to the docs.
                        fs::create_dir_all(dest_parent)?;
                    }
                }
                let to = dest.join(path.strip_prefix(source)?);
                if to.exists() {
                    let should_copy = match overwrite {
                        Overwrite::Always => true,
                        Overwrite::Ask => {
                            prompt(&format!("'{}' already exists. overwrite?", to.display()))
                        }
                        _ => false,
                    };
                    if should_copy {
                        fs::copy(path, to)?;
                    }
                } else {
                    fs::copy(path, to)?;
                }
            }

            anyhow::Ok(())
        })?;
    Ok(())
}

impl Deployer {
    #[tracing::instrument(skip_all, err)]
    pub fn deploy(
        &self,
        source: &Path,
        dest: &Path,
        location: &Location,
        mode: &CopyMode,
        overwrite: bool,
        remove_source: bool,
    ) -> Result<()> {
        // xxx: either way canonicalize paths.
        let final_source = source.join(location.subfolder.clone().unwrap_or_default());
        match mode {
            CopyMode::Copy => {
                if dest.exists() {
                    anyhow::bail!("path already exists: {}", dest.display());
                }
                std::fs::create_dir_all(dest)?;
                copy_dir(&final_source, dest, Overwrite::Always)?;
            }
            CopyMode::Apply => {
                std::fs::create_dir_all(dest)?;
                copy_dir(
                    &final_source,
                    dest,
                    if overwrite {
                        Overwrite::Always
                    } else {
                        Overwrite::Ask
                    },
                )?;
            }
        }
        if remove_source {
            println!("simulate remove {}", source.display());
        }
        // copy vs apply
        Ok(())
    }
}

fn prompt(q: &str) -> bool {
    dialoguer::Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(q)
        .interact()
        .unwrap()
}
