use anyhow::{bail, Context, Result};
use content_inspector::inspect;
use regex::Regex;
use serde_derive::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use tera::Tera;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Swap {
    #[serde(rename = "key")]
    pub key: String,

    #[serde(rename = "val_template")]
    pub val_template: Option<String>,

    #[serde(rename = "val")]
    pub val: Option<String>,

    #[serde(rename = "path")]
    #[serde(with = "serde_regex")]
    #[serde(default)]
    pub path: Option<Regex>,

    #[serde(default)]
    #[serde(rename = "kind")]
    pub kind: SwapKind,
}

impl Swap {
    fn match_path(&self, p: &Path) -> bool {
        match self.kind {
            SwapKind::Path | SwapKind::All => {
                let pstr = p.display().to_string();
                self.path.as_ref().map_or(true, |exp| exp.is_match(&pstr))
            }
            SwapKind::Content => false,
        }
    }

    fn match_content(&self, p: &Path) -> bool {
        match self.kind {
            SwapKind::Content | SwapKind::All => {
                let pstr = p.display().to_string();
                self.path.as_ref().map_or(true, |exp| exp.is_match(&pstr))
            }
            SwapKind::Path => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SwapKind {
    #[default]
    #[serde(rename = "all")]
    All,
    #[serde(rename = "content")]
    Content,
    #[serde(rename = "path")]
    Path,
}

pub struct Swapper {
    swaps: Vec<Swap>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct CopyResult {
    pub dest: PathBuf,
    pub op: SwapOp,
}
#[derive(Debug, Clone, Serialize, Default)]
pub enum SwapOp {
    #[default]
    Copied,
    Rendered(usize),
}

impl Swapper {
    ///
    /// Create a swapper with fully populated swaps
    ///
    /// # Errors
    /// Return errors when swaps cannot be populated, e.g. when a `val_template` is illegal
    pub fn with_vars(swaps: Option<&Vec<Swap>>, vars: &BTreeMap<String, String>) -> Result<Self> {
        let empty = vec![];
        let s = swaps.unwrap_or(&empty);
        Ok(Self {
            swaps: Self::materialize_val_template(s, vars)?,
        })
    }

    pub fn path(&self, p: &Path) -> PathBuf {
        let mut s = p.display().to_string();
        for swap in self.swaps.iter().filter(|s| s.match_path(p)) {
            if let Some(val) = swap.val.as_ref() {
                s = s.replace(swap.key.as_str(), val);
            }
        }
        PathBuf::from(s)
    }

    pub fn render_content<'a>(content_swaps: &[&Swap], original: &'a str) -> (Cow<'a, str>, usize) {
        let mut out: Cow<'_, str> = original.into();
        let mut count = 0;
        for swap in content_swaps {
            if let Some(val) = swap.val.as_ref() {
                count += out.matches(swap.key.as_str()).count();
                out = out.replace(swap.key.as_str(), val).into();
            }
        }
        (out, count)
    }

    /// Copy from `source` to `dest`, creating all folders if missing in `dest`
    ///
    /// # Errors
    ///
    /// This function will return an error if any of the `io` operations fail
    pub fn copy_to(&self, source: &Path, dest: &Path) -> Result<CopyResult> {
        let swapped = self.path(dest);
        let parent = swapped
            .parent()
            .ok_or_else(|| anyhow::anyhow!("cannot get parent for {:?}", swapped))?;
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        };

        let content_swaps = self
            .swaps
            .iter()
            .filter(|swap| swap.match_content(source))
            .collect::<Vec<_>>();

        // bail if no swaps
        if content_swaps.is_empty() {
            fs::copy(source, &swapped)?;
            return Ok(CopyResult {
                dest: swapped,
                op: SwapOp::Copied,
            });
        }

        // bail if text and has swaps
        let original = fs::read(source).with_context(|| format!("reading {}", source.display()))?;
        if inspect(&original).is_text() {
            let read = String::from_utf8(original)?;
            let (contents, count) = Self::render_content(&content_swaps, read.as_str());
            fs::write(&swapped, contents.as_bytes())?;
            return Ok(CopyResult {
                dest: swapped,
                op: SwapOp::Rendered(count),
            });
        }

        // turned out to be binary, we can't swap
        fs::copy(source, &swapped)?;
        Ok(CopyResult {
            dest: swapped,
            op: SwapOp::Copied,
        })
    }

    pub fn exists(&self, dest: &Path) -> bool {
        let p = self.path(dest);
        p.exists()
    }

    fn materialize_val_template(
        swaps: &[Swap],
        varbag: &BTreeMap<String, String>,
    ) -> Result<Vec<Swap>> {
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

        let swaps = Swapper::materialize_val_template(
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

        let swaps = Swapper::materialize_val_template(
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

        let swaps = Swapper::materialize_val_template(
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

        let swaps = Swapper::materialize_val_template(
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
        assert!(Swap::default().match_path(Path::new("some/path")));

        assert!(Swap {
            kind: SwapKind::All,
            ..Default::default()
        }
        .match_path(Path::new("some/path")));

        assert!(Swap {
            kind: SwapKind::Path,
            ..Default::default()
        }
        .match_path(Path::new("some/path")));

        assert!(!Swap {
            kind: SwapKind::Content,
            ..Default::default()
        }
        .match_path(Path::new("some/path")));

        assert!(!Swap {
            kind: SwapKind::Path,
            path: Some(Regex::new(".*foo").unwrap()),
            ..Default::default()
        }
        .match_path(Path::new("some/path")));

        assert!(Swap {
            kind: SwapKind::Path,
            path: Some(Regex::new("some/.*").unwrap()),
            ..Default::default()
        }
        .match_path(Path::new("some/path")));
    }

    #[test]
    fn test_match_copy() {
        assert!(Swap::default().match_content(Path::new("some/path")));

        assert!(Swap {
            kind: SwapKind::All,
            ..Default::default()
        }
        .match_content(Path::new("some/path")));

        assert!(!Swap {
            kind: SwapKind::Path,
            ..Default::default()
        }
        .match_content(Path::new("some/path")));

        assert!(Swap {
            kind: SwapKind::Content,
            ..Default::default()
        }
        .match_content(Path::new("some/path")));

        assert!(!Swap {
            kind: SwapKind::Content,
            path: Some(Regex::new(".*foo").unwrap()),
            ..Default::default()
        }
        .match_content(Path::new("some/path")));

        assert!(Swap {
            kind: SwapKind::Content,
            path: Some(Regex::new("some/.*").unwrap()),
            ..Default::default()
        }
        .match_content(Path::new("some/path")));
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

    #[test]
    fn test_multiple_path_swaps() {
        let swaps = vec![
            Swap {
                key: "$SWAPME$".to_string(),
                kind: SwapKind::All,
                val_template: Some("{{greeting | kebab_case}}".to_string()),
                ..Default::default()
            },
            Swap {
                key: "HELLO".to_string(),
                kind: SwapKind::All,
                val: Some("world".to_string()),
                ..Default::default()
            },
        ];
        let mut vars = BTreeMap::new();
        vars.insert("greeting".into(), "hello world".into());

        let swapper = Swapper::with_vars(Some(&swaps), &vars).unwrap();
        assert_eq!(
            "some/hello/hello-world/world/path",
            swapper
                .path(Path::new("some/hello/$SWAPME$/HELLO/path"))
                .display()
                .to_string()
        );
    }

    #[test]
    fn test_multiple_content_swaps() {
        let swaps = vec![
            Swap {
                key: "better".to_string(),
                kind: SwapKind::All,
                val: Some("worse".to_string()),
                ..Default::default()
            },
            Swap {
                key: " don't".to_string(),
                kind: SwapKind::All,
                val: Some("".to_string()),
                ..Default::default()
            },
            Swap {
                key: "$USER$".to_string(),
                kind: SwapKind::All,
                val_template: Some("{{user}}".to_string()),
                ..Default::default()
            },
            Swap {
                key: "SWAP_ME".to_string(),
                kind: SwapKind::Path,
                val: Some("swapped".to_string()),
                ..Default::default()
            },
        ];
        let mut vars = BTreeMap::new();
        vars.insert("user".into(), "Johnny".into());

        let swapper = Swapper::with_vars(Some(&swaps), &vars).unwrap();
        let _r = fs::remove_file("tests-out/fixtures/content/swapped/hey.txt");

        swapper
            .copy_to(
                Path::new("tests/fixtures/content/SWAP_ME/hey.txt"),
                Path::new("tests-out/fixtures/content/SWAP_ME/hey.txt"),
            )
            .unwrap();
        assert_yaml_snapshot!(
            fs::read_to_string("tests-out/fixtures/content/swapped/hey.txt")
                .unwrap()
                .lines()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}
