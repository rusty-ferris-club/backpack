use crate::config::ProjectSetupActions;
use crate::data::{Location, Opts, Overwrite};
use crate::templates::{CopyResult, Swapper};
use crate::ui::Prompt;
use anyhow::Result;
use interactive_actions::{
    data::ActionResult,
    data::{Action, ActionHook},
    ActionRunner,
};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tracing::warn;
use walkdir;

#[derive(Debug, Serialize)]
pub struct Coordinate {
    pub from: PathBuf,
    pub to: PathBuf,
    pub is_file: bool,
    pub remove_from: bool,
}

impl Coordinate {
    /// Calculate copy coordinates from source, dest, and location to paths on disk: from, to
    ///
    /// # Errors
    ///
    /// This function will return an error if I/O fails
    pub fn new(
        source: &Path,
        dest: Option<&Path>,
        location: &Location,
        remove: bool,
    ) -> Result<Self> {
        // source | source + location.subfolder
        let from = location
            .subfolder
            .as_ref()
            .map_or_else(|| source.into(), |subfolder| source.join(subfolder));

        // is this "deploying" a single file or a folder?
        let is_file = from.is_file();

        let location_folder = location.subfolder.clone().map(PathBuf::from);
        let final_dest = if let Some(dest) = dest {
            if is_file {
                let fname = from
                    .file_name()
                    .ok_or_else(|| anyhow::anyhow!("cannot get file name for {:?}", from))?;

                dest.join(fname)
            } else {
                dest.to_path_buf()
            }
        } else if is_file {
            let fname = from
                .file_name()
                .ok_or_else(|| anyhow::anyhow!("cannot get file name for {:?}", from))?;

            // location was: github.com/foo/baz/-/[tools/ci/bar.yaml], source: /tmp/foo/baz/tools/ci/bar.yaml, dest: tools/ci/bar.yaml
            //                                     `-- subfolder
            location_folder.unwrap_or_else(|| fname.into())
        } else {
            location_folder.unwrap_or_else(|| ".".into())
        };

        Ok(Self {
            from,
            to: final_dest,
            is_file,
            remove_from: remove,
        })
    }
}

pub struct Deployer<'a> {
    action_runner: &'a mut ActionRunner,
}

const DONT_COPY: &[&str] = &[".git", ".backpack-project.yml"];

impl<'a> Deployer<'a> {
    pub fn new(action_runner: &'a mut ActionRunner) -> Self {
        Self { action_runner }
    }

    #[tracing::instrument(skip_all, err)]
    pub fn deploy(
        &mut self,
        coord: Coordinate,
        project_setup: Option<ProjectSetupActions>,
        vars: &mut BTreeMap<String, String>,
        opts: &Opts,
        prompt: &mut Prompt<'_>,
    ) -> Result<(Vec<CopyResult>, Option<Vec<ActionResult>>)> {
        // xxx: either way canonicalize paths.
        let actions_dest = if coord.is_file {
            coord.to.parent().unwrap_or_else(|| Path::new("."))
        } else {
            coord.to.as_path()
        };

        let (actions, swaps) = project_setup
            .as_ref()
            .map_or((None, None), |p| (p.actions.as_ref(), p.swaps.as_ref()));

        if let Some(actions) = actions {
            self.action_runner.run(
                actions,
                Some(actions_dest),
                vars,
                ActionHook::Before,
                None::<fn(&Action)>,
            )?;
        }

        let swapper = Swapper::with_vars(swaps, vars)?;

        let files = self.copy(
            &swapper,
            &coord.from,
            &coord.to,
            coord.is_file,
            if opts.overwrite {
                Overwrite::Always
            } else {
                Overwrite::Ask
            },
            prompt,
        )?;

        if coord.remove_from {
            // xxx don't remove for now
            warn!(
                "remove requested, but not removing '{}'",
                coord.from.display()
            );
        }

        let after_actions = if let Some(actions) = actions {
            Some(self.action_runner.run(
                actions,
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
            .filter_entry(|entry| {
                let path = entry.path();
                !DONT_COPY.iter().any(|c| path.ends_with(c))
            })
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

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_yaml_snapshot;
    use url::Url;

    #[test]
    fn test_coord_new() {
        let norm_paths =
            || insta::dynamic_redaction(|value, _path| value.as_str().unwrap().replace('\\', "/"));

        assert_yaml_snapshot!(Coordinate::new(
            Path::new("here"),
            None,
            &Location::from(&Url::parse("https://github.com/foo/bar").unwrap(), true).unwrap(),
            false
        ).unwrap(),{ ".from" => norm_paths() });

        assert_yaml_snapshot!(Coordinate::new(
            Path::new("here"),
            None,
            &Location::from(
                &Url::parse("https://github.com/foo/bar/-/subfolder/qux").unwrap(),
                true
            )
            .unwrap(),
            false
        ).unwrap(),{ ".from" => norm_paths() });

        assert_yaml_snapshot!(Coordinate::new(
            Path::new("here"),
            Some(Path::new("there")),
            &Location::from(&Url::parse("https://github.com/foo/bar").unwrap(), true).unwrap(),
            false
        ).unwrap(),{ ".from" => norm_paths() });

        assert_yaml_snapshot!(Coordinate::new(
            Path::new("here"),
            Some(Path::new("there")),
            &Location::from(
                &Url::parse("https://github.com/foo/bar/-/subfolder/qux").unwrap(),
                true
            )
            .unwrap(),
            false
        ).unwrap(),{ ".from" => norm_paths() });

        assert_yaml_snapshot!(Coordinate::new(
            Path::new("tests"),
            None,
            &Location::from(
                &Url::parse("https://github.com/foo/bar/-/fixtures/local-project.yaml").unwrap(),
                true
            )
            .unwrap(),
            false
        ).unwrap(),{ ".from" => norm_paths() });

        assert_yaml_snapshot!(Coordinate::new(
            Path::new("tests"),
            Some(Path::new("there")),
            &Location::from(
                &Url::parse("https://github.com/foo/bar/-/fixtures/local-project.yaml").unwrap(),
                true
            )
            .unwrap(),
            false
        ).unwrap(),{ ".from" => norm_paths() });
    }
}
