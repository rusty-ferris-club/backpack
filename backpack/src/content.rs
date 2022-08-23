use crate::data::{CopyMode, Location, Opts, Overwrite};
use crate::templates::{CopyResult, Swap, Swapper};
use crate::ui::Prompt;
use anyhow::Result;
use interactive_actions::{
    data::ActionResult,
    data::{Action, ActionHook},
    ActionRunner,
};
use std::collections::BTreeMap;
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
        swaps: Option<&Vec<Swap>>,
        vars: &mut BTreeMap<String, String>,
        opts: &Opts,
        prompt: &mut Prompt<'_>,
    ) -> Result<(Vec<CopyResult>, Option<Vec<ActionResult>>)> {
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

        let actions_dest = if is_file {
            final_dest.parent().unwrap_or_else(|| Path::new("."))
        } else {
            final_dest.as_path()
        };

        let swapper = Swapper::with_vars(swaps, vars)?;
        let files = match opts.mode {
            CopyMode::Copy => {
                if final_dest.exists() {
                    anyhow::bail!("path already exists: {}", final_dest.display());
                }
                self.copy(
                    &swapper,
                    &final_source,
                    &final_dest,
                    is_file,
                    Overwrite::Always,
                    prompt,
                )?
            }
            CopyMode::Apply => self.copy(
                &swapper,
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

        let after_actions = if let Some(action_runner) = action_runner.as_mut() {
            Some(action_runner.run(
                Some(actions_dest),
                vars,
                ActionHook::After,
                Some(|action: &Action| prompt.say_action(action.name.as_str())),
            )?)
        } else {
            None
        };

        Ok((files, after_actions))
    }

    #[tracing::instrument(skip_all, err)]
    fn copy(
        &self,
        swapper: &Swapper,
        source: &Path,
        dest: &Path,
        is_file: bool,
        overwrite: Overwrite,
        prompt: &mut Prompt<'_>,
    ) -> Result<Vec<CopyResult>> {
        if is_file {
            return Ok(vec![swapper.copy_to(source, dest)?]);
        }

        let mut copied = vec![];
        walkdir::WalkDir::new(source)
            .into_iter()
            .try_for_each(|entry| {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    let to = dest.join(path.strip_prefix(source)?);
                    let to_path = to.as_path();

                    //
                    // trading off some inefficiency for readability here.
                    // the swapped path can be created only once, but instead it's created
                    // many times below in `exists`, in prompt, and by each of the `copy`s
                    // instead of creating a swapped path and *remembering* to pass throughout the workflow,
                    // we hide its creation.
                    //
                    // in addition, each `copy` will check to see if the parent exists, and if not, create it.
                    // we could create the parent just once here, but again - hiding it inside swapper is better
                    // than remembering nuances in the prices of checking parent folder more times than needed.
                    //
                    if swapper.exists(to_path) {
                        let should_copy = match overwrite {
                            Overwrite::Always => true,
                            Overwrite::Ask => prompt
                                .confirm_overwrite(swapper.path(to_path).as_path())
                                .unwrap_or(false),
                            _ => false,
                        };
                        if should_copy {
                            let to_swapped = swapper.copy_to(path, to_path)?;
                            copied.push(to_swapped);
                        }
                    } else {
                        let to_swapped = swapper.copy_to(path, to_path)?;
                        copied.push(to_swapped);
                    }
                }

                anyhow::Ok(())
            })?;
        Ok(copied)
    }
}
