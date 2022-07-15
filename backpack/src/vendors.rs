use crate::{
    config::{Config, CustomVendor},
    data::{Archive, ArchiveRoot, Assets, Location},
    git::GitProvider,
};
use anyhow::Result as AnyResult;
use core::fmt::Debug;
use tracing;
pub struct Vendors<'a> {
    config: &'a Config,
}

impl<'a> Vendors<'a> {
    pub const fn new(config: &'a Config) -> Self {
        Self { config }
    }

    #[tracing::instrument(name = "lookup_vendor", skip(self), err)]
    pub fn lookup(&self, vendor: &str) -> AnyResult<Box<dyn Vendor>> {
        let github = CustomVendor {
            kind: "github".to_string(),
            base: "github.com".to_string(),
        };
        let v = if vendor.is_empty() {
            self.config
                .vendors
                .as_ref()
                .and_then(|v| v.vendors_default.as_ref())
                .or(Some(&github))
        } else {
            self.config
                .vendors
                .as_ref()
                .and_then(|vendors| vendors.custom.as_ref())
                .and_then(|h| h.get(vendor))
        };
        v.map_or_else(
            || Vendors::lookup_table(vendor, None),
            |v| Vendors::lookup_table(&v.kind, Some(v.base.as_ref())),
        )
    }
    fn lookup_table(token: &str, base: Option<&str>) -> AnyResult<Box<dyn Vendor>> {
        match token {
            "gh" | "github.com" | "github" => Ok(Box::new(Github::new(base))),
            "gl" | "gitlab.com" | "gitlab" => Ok(Box::new(Gitlab::new(base))),
            "bb" | "bitbucket.org" | "bitbucket" => Ok(Box::new(BitBucket::new(base))),
            _ => anyhow::bail!("no vendor found for: {}", token),
        }
    }
}

pub trait Vendor {
    fn base(&self) -> &str;

    /// Resolve a location's assets.
    /// For example, a location my describe a github repo, and a branch, and its assets
    /// will be where you can download the code of that branch as a tarball.
    ///
    /// # Errors
    ///
    /// This function will return an error if network or other I/O is erroring.
    fn resolve(&self, location: &Location, git: &dyn GitProvider) -> AnyResult<(Location, Assets)>;
}

impl Debug for dyn Vendor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.base())
    }
}

pub struct Github {
    base: String,
}

impl Github {
    pub fn new(base: Option<&str>) -> Self {
        Self {
            base: base.map_or_else(|| "github.com".to_string(), ToString::to_string),
        }
    }
}
impl Vendor for Github {
    fn base(&self) -> &str {
        self.base.as_str()
    }
    #[tracing::instrument(name = "github_resolve", skip_all, err)]
    fn resolve(&self, location: &Location, git: &dyn GitProvider) -> AnyResult<(Location, Assets)> {
        let gref = git.get_ref_or_default(location)?.ref_;
        Ok((
            location.clone(),
            Assets {
                archive: Some(Archive {
                    url: format!(
                        "https://{}{}/archive/{}.tar.gz",
                        self.base(),
                        location.path,
                        gref,
                    ),
                    root: ArchiveRoot::FirstFolder,
                }),
                git: Some(format!(
                    "git@{}:{}.git",
                    self.base(),
                    location.path.trim_start_matches('/')
                )),
            },
        ))
    }
}
pub struct BitBucket {
    base: String,
}

impl BitBucket {
    pub fn new(base: Option<&str>) -> Self {
        Self {
            base: base.map_or_else(|| "bitbucket.org".to_string(), ToString::to_string),
        }
    }
}

impl Vendor for BitBucket {
    fn base(&self) -> &str {
        self.base.as_str()
    }

    #[tracing::instrument(name = "bitbucket_resolve", skip_all, err)]
    fn resolve(&self, location: &Location, git: &dyn GitProvider) -> AnyResult<(Location, Assets)> {
        let gref = git.get_ref_or_default(location)?.ref_;
        Ok((
            location.clone(),
            Assets {
                archive: Some(Archive {
                    url: format!(
                        "https://{}{}/get/{}.tar.gz",
                        self.base(),
                        location.path,
                        gref,
                    ),
                    root: ArchiveRoot::FirstFolder,
                }),
                git: Some(format!(
                    "git@{}:{}.git",
                    self.base(),
                    location.path.trim_start_matches('/')
                )),
            },
        ))
    }
}

pub struct Gitlab {
    base: String,
}

impl Gitlab {
    pub fn new(base: Option<&str>) -> Self {
        Self {
            base: base.map_or_else(|| "gitlab.com".to_string(), ToString::to_string),
        }
    }
}

impl Vendor for Gitlab {
    fn base(&self) -> &str {
        self.base.as_str()
    }

    #[tracing::instrument(name = "gitlab_resolve", skip_all, err)]
    fn resolve(&self, location: &Location, git: &dyn GitProvider) -> AnyResult<(Location, Assets)> {
        let gref = git.get_ref_or_default(location)?.ref_;
        let gref_file = gref.replace('/', "-");
        Ok((
            location.clone(),
            Assets {
                archive: Some(Archive {
                    url: format!(
                        "https://{}{}/-/archive/{}/{}-{}.tar.gz",
                        self.base(),
                        location.path,
                        gref,
                        location.project,
                        gref_file,
                    ),
                    root: ArchiveRoot::FirstFolder,
                }),
                git: Some(format!(
                    "git@{}:{}.git",
                    self.base(),
                    location.path.trim_start_matches('/')
                )),
            },
        ))
    }
}