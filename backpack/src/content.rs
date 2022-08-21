use crate::config::{Swap, SwapKind};
use crate::data::{CopyMode, Location, Opts, Overwrite};
use crate::ui::Prompt;
use anyhow::{bail, Result};
use interactive_actions::{
    data::ActionResult,
    data::{Action, ActionHook},
    ActionRunner,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use tera::Tera;
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

        let actions_dest = if is_file {
            final_dest.parent().unwrap_or_else(|| Path::new("."))
        } else {
            final_dest.as_path()
        };

        let before_actions = if let Some(action_runner) = action_runner.as_mut() {
            Some(action_runner.run(
                Some(actions_dest),
                vars,
                ActionHook::Before,
                Some(|action: &Action| prompt.say_action(action.name.as_str())),
            )?)
        } else {
            None
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

        // just a fancy zip + fold of the two collections
        let actions = [before_actions, after_actions]
            .into_iter()
            .flatten()
            .reduce(|mut a, mut b| {
                a.append(&mut b);
                a
            });

        Ok((files, actions))
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
    ) -> Result<Vec<String>> {
        // swapfs = Swaps::new
        if is_file {
            let swapped = swapper.copy_to(source, dest)?;
            return Ok(vec![swapped.display().to_string()]);
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
                            copied.push(to_swapped.display().to_string());
                        }
                    } else {
                        let to_swapped = swapper.copy_to(path, to_path)?; //swapfs.copy(..)
                        copied.push(to_swapped.display().to_string());
                    }
                }

                anyhow::Ok(())
            })?;
        Ok(copied)
    }
}

pub struct Swapper {
    swaps: Vec<Swap>,
}
impl Swapper {
    ///
    /// Create a swapper with fully populated swaps
    ///
    /// # Errors
    /// Return errors when swaps cannot be populated, e.g. when a `val_template` is illegal
    #[must_use]
    pub fn with_vars(swaps: Option<&Vec<Swap>>, vars: &BTreeMap<String, String>) -> Result<Self> {
        let empty = vec![];
        let s = swaps.unwrap_or(&empty);
        Ok(Self {
            swaps: render_swaps(s, vars)?,
        })
    }

    pub fn path(&self, p: &Path) -> PathBuf {
        let pstr = p.display().to_string();
        let mut s = pstr.clone();
        for swap in &self.swaps {
            if match_path(pstr.as_str(), swap) {
                if let Some(val) = swap.val.as_ref() {
                    s = s.replace(swap.key.as_str(), val);
                }
            }
        }
        PathBuf::from(s)
    }
    pub fn copy_to(&self, source: &Path, dest: &Path) -> Result<PathBuf> {
        let swapped = self.path(dest);
        let parent = dest
            .parent()
            .ok_or_else(|| anyhow::anyhow!("cannot get parent for {:?}", dest))?;
        if !parent.exists() {
            fs::create_dir_all(&parent)?;
        };
        fs::copy(source, &swapped)?; // swapfs.copy(s, d)
        Ok(swapped)
    }
    pub fn exists(&self, dest: &Path) -> bool {
        let p = self.path(dest);
        p.exists()
    }
}

fn render_swaps(swaps: &[Swap], varbag: &BTreeMap<String, String>) -> Result<Vec<Swap>> {
    let mut tera = Tera::default();
    tera_text_filters::register_all(&mut tera);
    let context = tera::Context::from_serialize(varbag)?;
    swaps
        .iter()
        .map(|swap| {
            let val = match (swap.val.as_ref(), swap.val_template.as_ref()) {
                (Some(v), _) => v.clone(),
                (None, Some(v)) => tera.render_str(v, &context)?,
                (None, None) => bail!("each swap should have either `val` or `val_template`"),
            };
            let mut s = swap.clone();
            s.val = Some(val);
            Ok(s)
        })
        .collect::<Result<Vec<_>>>()
}

fn match_path(p: &str, swap: &Swap) -> bool {
    match swap.kind {
        SwapKind::Path | SwapKind::All => swap.path.as_ref().map_or(true, |exp| exp.is_match(p)),
        SwapKind::Content => false,
    }
}

fn match_copy(p: &str, swap: &Swap) -> bool {
    match swap.kind {
        SwapKind::Content | SwapKind::All => swap.path.as_ref().map_or(true, |exp| exp.is_match(p)),
        SwapKind::Path => false,
    }
}

#[cfg(test)]
mod tests {
    use regex::Regex;
    use std::vec;

    use insta::assert_yaml_snapshot;
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_render_swaps() {
        let mut h = BTreeMap::new();
        h.insert("world".to_string(), "crewl world".to_string());

        let swaps = render_swaps(
            vec![Swap {
                key: "key".to_string(),
                val_template: Some("Hello {{world}}".to_string()),
                val: None,
                ..Default::default()
            }]
            .as_slice(),
            &h,
        )
        .unwrap();
        assert_yaml_snapshot!(swaps);
    }

    #[test]
    fn test_render_swaps_inflections() {
        let mut h = BTreeMap::new();
        h.insert("world".to_string(), "crewl world".to_string());

        let swaps = render_swaps(
            vec![Swap {
                key: "key".to_string(),
                val_template: Some("Hello {{world | kebab_case}}".to_string()),
                val: None,
                ..Default::default()
            }]
            .as_slice(),
            &h,
        )
        .unwrap();
        assert_yaml_snapshot!(swaps);
    }

    #[test]
    fn test_render_swaps_empty_context() {
        let h = BTreeMap::new();

        let swaps = render_swaps(
            vec![Swap {
                key: "key".to_string(),
                val_template: Some("Hello {{world}}".to_string()),
                val: None,
                ..Default::default()
            }]
            .as_slice(),
            &h,
        );
        assert_eq!(
            swaps.unwrap_err().to_string(),
            "Failed to render '__tera_one_off'"
        );
    }

    #[test]
    fn test_render_wrong_context() {
        let mut h = BTreeMap::new();
        h.insert("foobar".to_string(), "crewl world".to_string());

        let swaps = render_swaps(
            vec![Swap {
                key: "key".to_string(),
                val_template: Some("Hello {{world}}".to_string()),
                val: None,
                ..Default::default()
            }]
            .as_slice(),
            &h,
        );

        assert_eq!(
            swaps.unwrap_err().to_string(),
            "Failed to render '__tera_one_off'"
        );
    }

    #[test]
    fn test_match_path() {
        assert!(match_path("some/path", &Swap::default()));
        assert!(match_path(
            "some/path",
            &Swap {
                kind: SwapKind::All,
                ..Default::default()
            }
        ));
        assert!(match_path(
            "some/path",
            &Swap {
                kind: SwapKind::Path,
                ..Default::default()
            }
        ));
        assert!(!match_path(
            "some/path",
            &Swap {
                kind: SwapKind::Content,
                ..Default::default()
            }
        ));
        assert!(!match_path(
            "some/path",
            &Swap {
                kind: SwapKind::Path,
                path: Some(Regex::new(".*foo").unwrap()),
                ..Default::default()
            }
        ));
        assert!(match_path(
            "some/path",
            &Swap {
                kind: SwapKind::Path,
                path: Some(Regex::new("some/.*").unwrap()),
                ..Default::default()
            }
        ));
    }

    #[test]
    fn test_match_copy() {
        assert!(match_copy("some/path", &Swap::default()));
        assert!(match_copy(
            "some/path",
            &Swap {
                kind: SwapKind::All,
                ..Default::default()
            }
        ));
        assert!(!match_copy(
            "some/path",
            &Swap {
                kind: SwapKind::Path,
                ..Default::default()
            }
        ));
        assert!(match_copy(
            "some/path",
            &Swap {
                kind: SwapKind::Content,
                ..Default::default()
            }
        ));
        assert!(!match_copy(
            "some/path",
            &Swap {
                kind: SwapKind::Content,
                path: Some(Regex::new(".*foo").unwrap()),
                ..Default::default()
            }
        ));
        assert!(match_copy(
            "some/path",
            &Swap {
                kind: SwapKind::Content,
                path: Some(Regex::new("some/.*").unwrap()),
                ..Default::default()
            }
        ));
    }

    #[test]
    fn test_swap_path() {
        let swaps = vec![Swap {
            key: "$SWAPME$".to_string(),
            kind: SwapKind::All,
            val_template: Some("{{greeting | kebab_case}}".to_string()),
            ..Default::default()
        }];
        let mut vars = BTreeMap::new();
        vars.insert("greeting".into(), "hello world".into());

        let swapper = Swapper::with_vars(Some(&swaps), &vars).unwrap();
        assert_eq!(
            "some/hello-world/path",
            swapper
                .path(Path::new("some/$SWAPME$/path"))
                .display()
                .to_string()
        );
        assert_eq!(
            "some/naive/path",
            swapper
                .path(Path::new("some/naive/path"))
                .display()
                .to_string()
        );
    }
}
