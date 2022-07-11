use backpack::data::Location;
use backpack::git::{GitCmd, GitProvider};
use pretty_assertions::assert_eq;
use url::Url;

#[test]
fn test_mainbranch_resolving() {
    let git = GitCmd::default();
    let location = Location::from(
        &Url::parse("https://github.com/jondot/hygen").unwrap(),
        false,
    )
    .unwrap();
    let res = git.get_ref_or_default(&location).unwrap().ref_;
    assert_eq!(res, "refs/heads/master");
}

#[test]
fn test_tags_resolving() {
    let git = GitCmd::default();
    let location = Location::from(
        &Url::parse("https://github.com/jondot/hygen#v6.2.0").unwrap(),
        false,
    )
    .unwrap();
    let res = git.get_ref_or_default(&location).unwrap().ref_;
    assert_eq!(res, "refs/tags/v6.2.0");
}

#[test]
fn test_branches_resolving() {
    let git = GitCmd::default();
    let location = Location::from(
        &Url::parse("https://github.com/jondot/hygen#gh-pages").unwrap(),
        false,
    )
    .unwrap();
    let res = git.get_ref_or_default(&location).unwrap().ref_;
    assert_eq!(res, "refs/heads/gh-pages");
}
