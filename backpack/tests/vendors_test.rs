use backpack::data::Location;
use backpack::git::{GitProvider, RemoteInfo};
use backpack::vendors::{BitBucket, Github, Gitlab, Vendor};
use insta::assert_debug_snapshot;
use pretty_assertions::assert_eq;
use reqwest::{self, StatusCode};
use url::Url;

struct TestGitProvider {
    remote: RemoteInfo,
}
impl GitProvider for TestGitProvider {
    fn get_ref_or_default(
        &self,
        _location: &Location,
    ) -> anyhow::Result<backpack::git::RemoteInfo> {
        Ok(self.remote.clone())
    }
    fn ls_remote(&self, _location: &Location) -> anyhow::Result<Vec<backpack::git::RemoteInfo>> {
        Ok(vec![])
    }
    fn shallow_clone(&self, _location: &Location, _out: &str) -> anyhow::Result<()> {
        Ok(())
    }
    fn get_local_url(&self) -> anyhow::Result<String> {
        Ok(String::new())
    }
}

#[test]
fn test_github() {
    let git = TestGitProvider {
        remote: RemoteInfo {
            revision: "rev".to_string(),
            ref_: "refs/heads/master".to_string(),
        },
    };
    let gh = Github::new(None);
    let location = Location::from(
        &Url::parse("https://github.com/jondot/hygen").unwrap(),
        false,
    )
    .unwrap();
    let assets = gh.resolve(&location, &git).unwrap();
    assert_debug_snapshot!(assets);
    assert_eq!(
        reqwest::blocking::get(assets.archive.unwrap().url)
            .unwrap()
            .status(),
        StatusCode::OK
    );
}

#[test]
fn test_gitlab() {
    let git = TestGitProvider {
        remote: RemoteInfo {
            revision: "rev".to_string(),
            ref_: "refs/heads/master".to_string(),
        },
    };
    let gh = Gitlab::new(None);
    let location = Location::from(
        &Url::parse("https://gitlab.com/jondot/backpack-e2e").unwrap(),
        false,
    )
    .unwrap();
    let assets = gh.resolve(&location, &git).unwrap();
    assert_debug_snapshot!(assets);
    assert_eq!(
        reqwest::blocking::get(assets.archive.unwrap().url)
            .unwrap()
            .status(),
        StatusCode::OK
    );
}

#[test]
fn test_bitbucket() {
    let git = TestGitProvider {
        remote: RemoteInfo {
            revision: "rev".to_string(),
            ref_: "refs/heads/master".to_string(),
        },
    };
    let gh = BitBucket::new(None);
    let location = Location::from(
        &Url::parse("https://bitbucket.org/microsoft/azure-cli-run").unwrap(),
        false,
    )
    .unwrap();
    let assets = gh.resolve(&location, &git).unwrap();
    assert_debug_snapshot!(assets);
    assert_eq!(
        reqwest::blocking::get(assets.archive.unwrap().url)
            .unwrap()
            .status(),
        StatusCode::OK
    );
}
