use crate::data::{CopyMode, Location, Opts, Overwrite};
use crate::ui::Prompt;
use anyhow::Result;
use interactive_actions::{data::Action, data::ActionResult, ActionRunner};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::warn;
use walkdir;

pub struct Coordinate<'a> {
    pub source: &'a Path,
    pub dest: Option<&'a Path>,
    pub location: &'a Location,
    pub remove_source: bool,
}

#[derive(Default)]
pub struct Deployer {}

impl Deployer {
    #[tracing::instrument(skip_all, err)]
    pub fn deploy(
        &self,
        coord: Coordinate<'_>,
        mut action_runner: Option<ActionRunner<'_>>,
        opts: &Opts,
        prompt: &mut Prompt<'_>,
    ) -> Result<(Vec<String>, Option<Vec<ActionResult>>)> {
        // xxx: either way canonicalize paths.
        let final_source = coord
            .source
            .join(coord.location.subfolder.clone().unwrap_or_default());
        let dest = coord.dest.map(std::path::Path::to_path_buf);
        let location_path = coord.location.subfolder.clone().map(PathBuf::from);

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

        let files = match opts.mode {
            CopyMode::Copy => {
                if final_dest.exists() {
                    anyhow::bail!("path already exists: {}", final_dest.display());
                }
                self.copy(
                    &final_source,
                    &final_dest,
                    is_file,
                    Overwrite::Always,
                    prompt,
                )?
            }
            CopyMode::Apply => self.copy(
                &final_source,
                &final_dest,
                is_file,
                if opts.overwrite {
                    Overwrite::Always
                } else {
                    Overwrite::Ask
                },
                prompt,
            )?,
            CopyMode::All => {
                vec![]
            }
        };
        if coord.remove_source {
            // xxx don't remove for now
            warn!(
                "remove requested, but not removing '{}'",
                coord.source.display()
            );
        }

        let actions = if let Some(action_runner) = action_runner.as_mut() {
            let actions_dest = if is_file {
                final_dest.parent().unwrap_or_else(|| Path::new("."))
            } else {
                final_dest.as_path()
            };

            Some(action_runner.run(
                Some(actions_dest),
                Some(|action: &Action| prompt.say_action(action.name.as_str())),
            )?)
        } else {
            None
        };

        // copy vs apply
        Ok((files, actions))
    }

    #[tracing::instrument(skip_all, err)]
    fn copy(
        &self,
        source: &Path,
        dest: &Path,
        is_file: bool,
        overwrite: Overwrite,
        prompt: &mut Prompt<'_>,
    ) -> Result<Vec<String>> {
        if is_file {
            // dest is a full path incl. file
            let dest_path = dest
                .parent()
                .ok_or_else(|| anyhow::anyhow!("cannot get parent for {:?}", dest))?;
            if !dest_path.exists() {
                fs::create_dir_all(&dest_path)?;
            }

            fs::copy(source, &dest)?;
            return Ok(vec![dest.display().to_string()]);
        }

        let mut copied = vec![];
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
                                prompt.confirm_overwrite(to.as_path()).unwrap_or(false)
                            }
                            _ => false,
                        };
                        if should_copy {
                            fs::copy(path, &to)?;
                            copied.push(to.display().to_string());
                        }
                    } else {
                        fs::copy(path, &to)?;
                        copied.push(to.display().to_string());
                    }
                }

                anyhow::Ok(())
            })?;
        Ok(copied)
    }
}
