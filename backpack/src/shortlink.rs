use crate::{
    config::Config,
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

#[allow(clippy::type_complexity)]
fn expand<'a>(
    shortlink: &str,
    is_git: bool,
    config: &'a Config,
) -> AnyResult<(Box<dyn Vendor>, Location, Option<&'a Vec<Action>>)> {
    let (shortlink, is_git, actions) = config
        .projects
        .as_ref()
        .and_then(|projects| projects.get(shortlink))
        .map_or_else(
            || (shortlink, is_git, None),
            |project| {
                (
                    project.shortlink.as_str(),
                    project.is_git.unwrap_or(false),
                    project.actions.as_ref(),
                )
            },
        );
    let vendors = Vendors::new(config);
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
    Ok((vendor, location, actions))
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
    pub fn resolve(
        &self,
        shortlink: &str,
        is_git: bool,
    ) -> AnyResult<(Location, Assets, Option<&'a Vec<Action>>)> {
        let (vendor, location, actions) = expand(shortlink, is_git, self.config)?;
        let (location, assets) = vendor.resolve(&location, self.git)?;
        Ok((location, assets, actions))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    use insta::assert_debug_snapshot;

    macro_rules! set_snapshot_suffix {
        ($($expr:expr),*) => {{
            let mut settings = insta::Settings::clone_current();
            settings.set_prepend_module_to_snapshot(false);
            settings.set_snapshot_suffix(format!($($expr,)*));
            settings.bind_to_thread();
        }}
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
        assert_debug_snapshot!((link.clone(), expand(&link, false, &config)));
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
        let config = Config::default();
        assert_debug_snapshot!((link.clone(), expand(&link, false, &config)));
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
        let config = Config::default();
        assert_debug_snapshot!((link.clone(), expand(&link, false, &config)));
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
        let config = Config::default();
        assert_debug_snapshot!((link.clone(), expand(&link, false, &config)));
    }
}
