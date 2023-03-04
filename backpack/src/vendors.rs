use crate::{
    config::{CustomVendor, VendorsConfig},
    data::{Archive, ArchiveRoot, Assets, Location},
    git::GitProvider,
};
use anyhow::Result as AnyResult;
use core::fmt::Debug;
use tracing;
pub struct Vendors<'a> {
    config: Option<&'a VendorsConfig>,
}

impl<'a> Vendors<'a> {
    pub const fn new(config: Option<&'a VendorsConfig>) -> Self {
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
                .and_then(|v| v.vendors_default.as_ref())
                .or(Some(&github))
        } else {
            self.config
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
            "gist.github.com" | "gist" => Ok(Box::new(GithubGist::new(base))),
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
    fn resolve(&self, location: &Location, git: &dyn GitProvider) -> AnyResult<Assets>;
}

impl Debug for dyn Vendor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.base())
    }
}

// add gist vendor
// the new release should include single file support + gist
// find a way to test single file cloning

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
    fn resolve(&self, location: &Location, git: &dyn GitProvider) -> AnyResult<Assets> {
        let gref = git.get_ref_or_default(location)?.ref_;
        Ok(Assets {
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
        })
    }
}

pub struct LocalGit {}

impl Vendor for LocalGit {
    fn base(&self) -> &str {
        "file"
    }
    #[tracing::instrument(name = "localgit_resolve", skip_all, err)]
    fn resolve(&self, location: &Location, _git: &dyn GitProvider) -> AnyResult<Assets> {
        Ok(Assets {
            archive: None,
            git: Some(location.url.clone()),
        })
    }
}

pub struct GithubGist {
    base: String,
}

impl GithubGist {
    pub fn new(base: Option<&str>) -> Self {
        Self {
            base: base.map_or_else(|| "gist.github.com".to_string(), ToString::to_string),
        }
    }
}
impl Vendor for GithubGist {
    fn base(&self) -> &str {
        self.base.as_str()
    }
    #[tracing::instrument(name = "github_gist_resolve", skip_all, err)]
    fn resolve(&self, location: &Location, git: &dyn GitProvider) -> AnyResult<Assets> {
        let refs = git.ls_remote(location)?;
        let head = refs
            .iter()
            .find(|r| r.ref_ == "HEAD")
            .ok_or_else(|| anyhow::anyhow!("no HEAD ref found"))?;

        Ok(Assets {
            archive: Some(Archive {
                url: format!(
                    //   https://gist.github.com/jondot/15086f59dab44f30bb10f82ca09f4887/archive/44a751f50ea93f92c2bc6332e4de770429862888.zip
                    "https://{}{}/archive/{}.tar.gz",
                    self.base(),
                    location.path,
                    head.ref_,
                ),
                root: ArchiveRoot::FirstFolder,
            }),
            git: Some(format!(
                "git@{}:{}.git",
                self.base(),
                location.path.trim_start_matches('/')
            )),
        })
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
    fn resolve(&self, location: &Location, git: &dyn GitProvider) -> AnyResult<Assets> {
        let gref = git.get_ref_or_default(location)?.ref_;
        Ok(Assets {
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
        })
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
    fn resolve(&self, location: &Location, git: &dyn GitProvider) -> AnyResult<Assets> {
        let gref = git.get_ref_or_default(location)?.ref_;
        let gref_file = gref.replace('/', "-");
        Ok(Assets {
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
        })
    }
}
