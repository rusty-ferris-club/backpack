use crate::{
    config::{Config, Swap},
    data::{Assets, Location},
    git::GitProvider,
    vendors::{Vendor, Vendors},
};
use anyhow::Result as AnyResult;
use interactive_actions::data::Action;
use lazy_static::lazy_static;
use regex::Regex;
use tracing;
use url::Url;

lazy_static! {
    static ref RE_GIT: Regex = Regex::new(r"^git@(.+?):(.+)$").unwrap();
    static ref RE_VENDOR: Regex = Regex::new(r"^([a-zA-Z0-9_-]+):(.+)$").unwrap();
}

fn expand<'a>(
    shortlink: &str,
    is_git: bool,
    vendors: &'a Vendors<'_>,
) -> AnyResult<(Box<dyn Vendor>, Location)> {
    let (vendor, url) = if shortlink.starts_with("https://") {
        //https://github.com/jondot/hygen/-/foobar

        let url = Url::parse(shortlink)?;
        let vendor = vendors.lookup(
            url.domain()
                .ok_or_else(|| anyhow::anyhow!("domain is missing"))?,
        )?;
        (vendor, url)
    } else if let Some(caps) = RE_GIT.captures(shortlink) {
        let domain = caps
            .get(1)
            .ok_or_else(|| anyhow::anyhow!("parse failed: no domain"))?
            .as_str();
        (
            vendors.lookup(domain)?,
            Url::parse(&format!(
                "https://{}/{}",
                domain,
                caps.get(2)
                    .ok_or_else(|| anyhow::anyhow!("parse failed: no path"))?
                    .as_str()
            ))?,
        )
    } else if let Some(caps) = RE_VENDOR.captures(shortlink) {
        let vendor = vendors.lookup(
            caps.get(1)
                .ok_or_else(|| anyhow::anyhow!("parse failed: no vendor"))?
                .as_str(),
        )?;
        let url = Url::parse(&format!(
            "https://{}/{}",
            vendor.base(),
            caps.get(2)
                .ok_or_else(|| anyhow::anyhow!("parse failed: no path"))?
                .as_str()
        ))?;

        (vendor, url)
    } else {
        let vendor = vendors.lookup("")?;
        let url = Url::parse(&format!("https://{}/{}", vendor.base(), shortlink))?;
        (vendor, url)
    };

    let location = Location::from(&url, is_git)?;
    Ok((vendor, location))
}

pub struct Shortlink<'a> {
    config: &'a Config,
    git: &'a dyn GitProvider,
}

impl<'a> Shortlink<'a> {
    pub fn new(config: &'a Config, git: &'a dyn GitProvider) -> Self {
        Self { config, git }
    }

    #[tracing::instrument(name = "shortlink_resolve", skip_all, err)]
    pub fn resolve(&self, shortlink: &str, is_git: bool) -> AnyResult<(Location, Assets)> {
        // try to get better settings from projects config:
        let (shortlink, is_git) = self.config.project(shortlink).map_or_else(
            || (shortlink, is_git),
            |project| (project.shortlink.as_str(), project.is_git.unwrap_or(false)),
        );

        // expand and resolve
        let vendors = Vendors::new(self.config.vendors.as_ref());
        let (vendor, location) = expand(shortlink, is_git, &vendors)?;
        let (location, assets) = vendor.resolve(&location, self.git)?;
        Ok((location, assets))
    }

    pub fn actions(&self, shortlink: &str) -> Option<&'a Vec<Action>> {
        self.config
            .project(shortlink)
            .and_then(|project| project.actions.as_ref())
    }
    pub fn swaps(&self, shortlink: &str) -> Option<&'a Vec<Swap>> {
        self.config
            .project(shortlink)
            .and_then(|project| project.swaps.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    use insta::assert_debug_snapshot;

    macro_rules! set_snapshot_suffix {
        ($($expr:expr),*) => {
            let mut settings = insta::Settings::clone_current();
            settings.set_prepend_module_to_snapshot(false);
            settings.set_snapshot_suffix(format!($($expr,)*));
            let _guard = settings.bind_to_scope();
        }
    }

    #[rstest]
    fn test_custom_config(
        #[values("", "gl:", "gh:", "ghe:", "notfound:")] vendor: &str,
        #[values("hygen", "jondot/hygen", "rust-starter")] slug: &str,
        #[values("")] gref: &str,
    ) {
        set_snapshot_suffix!(
            "[{}]-[{}]-[{}]",
            vendor.replace(':', ""),
            slug.replace('/', "_"),
            gref.replace('/', "_")
        );
        let config = Config::from_text(
            r###"
projects:
  rust-starter:
    shortlink: jondot/rust-starter
    is_git: true

vendors:
  default: 
    kind: gitlab
    base: my.gitlab.com

  custom:
    gh:
      kind: github
      base: beta.github.com/my-org
    ghe:
      kind: github
      base: github.enterprise.example.com
"###,
        )
        .unwrap();

        let link = format!("{}{}{}", vendor, slug, gref);
        let vendors = Vendors::new(config.vendors.as_ref());
        assert_debug_snapshot!((link.clone(), expand(&link, false, &vendors)));
    }

    #[rstest]
    fn test_smoke(
        #[values("", "gl:")] vendor: &str,
        #[values("jondot/hygen/-/subfolder")] slug: &str,
        #[values("", "#wip")] gref: &str,
    ) {
        set_snapshot_suffix!(
            "[{}]-[{}]-[{}]",
            vendor.replace(':', "_"),
            slug.replace('/', "_"),
            gref.replace('/', "_")
        );

        let link = format!("{}{}{}", vendor, slug, gref);
        let vendors = Vendors::new(None);
        assert_debug_snapshot!((link.clone(), expand(&link, false, &vendors)));
    }

    #[rstest]
    fn test_vendors(
        #[values(
            "",
            "gh:",
            "gl:",
            "bb:",
            "my-vendor:",
            "git@github.com:",
            "https://gitlab.com/"
        )]
        vendor: &str,
        #[values("jondot/hygen")] slug: &str,
        #[values("")] gref: &str,
    ) {
        set_snapshot_suffix!(
            "[{}]-[{}]-[{}]",
            vendor.replace(':', "_"),
            slug.replace('/', "_"),
            gref.replace('/', "_")
        );

        let link = format!("{}{}{}", vendor, slug, gref);
        let vendors = Vendors::new(None);
        assert_debug_snapshot!((link.clone(), expand(&link, false, &vendors)));
    }

    #[rstest]
    fn test_locations(
        #[values("")] vendor: &str,
        #[values("hygen", "jondot/hygen", "group/team/repo", "jondot/hygen/-/subfolder")]
        slug: &str,
        #[values("", "#wip")] gref: &str,
    ) {
        set_snapshot_suffix!(
            "[{}]-[{}]-[{}]",
            vendor.replace(':', "_"),
            slug.replace('/', "_"),
            gref.replace('/', "_")
        );

        let link = format!("{}{}{}", vendor, slug, gref);
        let vendors = Vendors::new(None);
        assert_debug_snapshot!((link.clone(), expand(&link, false, &vendors)));
    }
}
