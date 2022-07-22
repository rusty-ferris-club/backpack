use crate::data::CopyMode;
use crate::data::Location;
use crate::data::Overwrite;
use anyhow::Result;
use dialoguer;
use dialoguer::theme::ColorfulTheme;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::warn;
use walkdir;

#[derive(Default)]
pub struct Deployer {}

#[tracing::instrument(skip_all, err)]
fn copy(source: &Path, dest: &Path, is_file: bool, overwrite: Overwrite) -> Result<()> {
    if is_file {
        // dest is a full path incl. file
        let dest_path = dest
            .parent()
            .ok_or_else(|| anyhow::anyhow!("cannot get parent for {:?}", dest))?;
        if !dest_path.exists() {
            println!("{:?}", dest_path);
            fs::create_dir_all(&dest_path)?;
        }

        println!("copy {:?}, {:?}", source, dest);
        fs::copy(source, &dest)?;
        return Ok(());
    }
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
        dest: Option<&Path>,
        location: &Location,
        mode: &CopyMode,
        overwrite: bool,
        remove_source: bool,
    ) -> Result<String> {
        // xxx: either way canonicalize paths.
        let final_source = source.join(location.subfolder.clone().unwrap_or_default());
        let dest = dest.map(std::path::Path::to_path_buf);
        let location_path = location.subfolder.clone().map(PathBuf::from);

        // is this "deploying" a single file or a folder?
        let is_file = final_source.is_file();
        let final_dest = if is_file {
            // final dest = dest | location+fname | fname
            let fname = final_source
                .file_name()
                .ok_or_else(|| anyhow::anyhow!("cannot get file name for {:?}", final_source))?;
            dest.or_else(|| {
                location_path.and_then(|loc| loc.parent().map(|p| p.to_path_buf().join(fname)))
            })
            .unwrap_or_else(|| PathBuf::from(fname))
        } else {
            dest.or(location_path)
                .unwrap_or_else(|| PathBuf::from(".".to_string()))
        };

        match mode {
            CopyMode::Copy => {
                if final_dest.exists() {
                    anyhow::bail!("path already exists: {}", final_dest.display());
                }
                copy(&final_source, &final_dest, is_file, Overwrite::Always)?;
            }
            CopyMode::Apply => {
                copy(
                    &final_source,
                    &final_dest,
                    is_file,
                    if overwrite {
                        Overwrite::Always
                    } else {
                        Overwrite::Ask
                    },
                )?;
            }
            CopyMode::All => {}
        }
        if remove_source {
            // xxx don't remove for now
            warn!("remove requested, but not removing '{}'", source.display());
        }
        // copy vs apply
        Ok(final_dest.display().to_string())
    }
}

fn prompt(q: &str) -> bool {
    dialoguer::Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(q)
        .interact()
        .unwrap()
}
