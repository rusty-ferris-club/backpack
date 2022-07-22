use std::{env, fs, path::PathBuf};

use backpack::config::Config;
use backpack::data::CopyMode;
use backpack::run::{Opts, Runner};
use insta::assert_debug_snapshot;
use serial_test::serial;
use walkdir::{DirEntry, WalkDir};

fn ensure_no_config() {
    env::set_var("BP_FOLDER", ".backpack-none");
    env::set_var("BP_CONF", "none.yaml");

    let global_folder = Config::global_config_folder().unwrap();
    assert!(global_folder.ends_with(".backpack-none"));
    if global_folder.exists() {
        fs::remove_dir_all(&global_folder).unwrap();
    }
}

fn list_folder(dest: &str) -> Vec<PathBuf> {
    let mut r = WalkDir::new(dest)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|file| file.metadata().unwrap().is_file())
        .map(DirEntry::into_path)
        .collect::<Vec<_>>();
    r.sort();
    r
}

fn run(shortlink: Option<&str>, dest: Option<&str>, mode: CopyMode) -> Vec<PathBuf> {
    let tests_out = "tests-out";
    fs::remove_dir_all(tests_out).ok();
    fs::create_dir(tests_out).unwrap();
    let current_dir = env::current_dir().unwrap();
    env::set_current_dir(tests_out).unwrap();

    Runner::default()
        .run(
            shortlink,
            dest,
            &Opts {
                overwrite: false,
                is_git: false,
                no_cache: false,
                no_dest_input: true,
                always_yes: true,
                remote: None,
                mode,
            },
        )
        .unwrap();

    env::set_current_dir(current_dir).unwrap();
    list_folder(tests_out)
}

fn run_with_no_config(shortlink: Option<&str>, dest: Option<&str>, mode: CopyMode) -> Vec<PathBuf> {
    ensure_no_config();
    run(shortlink, dest, mode)
}

#[test]
#[serial]
fn test_run_source_dest() {
    assert_debug_snapshot!(run_with_no_config(
        Some("rusty-ferris-club/backpack-e2e-frozen"),
        Some("out"),
        CopyMode::Copy,
    ));
}

#[test]
#[serial]
fn test_run_source_dest_subfolder() {
    assert_debug_snapshot!(run_with_no_config(
        Some("rusty-ferris-club/backpack-e2e-frozen/-/.github"),
        Some("out"),
        CopyMode::Copy,
    ));
}

#[test]
#[serial]
fn test_run_source_dest_single_file() {
    assert_debug_snapshot!(run_with_no_config(
        Some("rusty-ferris-club/backpack-e2e-frozen/-/.github/workflows/build.yml"),
        Some("out/build.yml"),
        CopyMode::Copy,
    ));
}

#[test]
#[serial]
fn test_run_source_single_file() {
    assert_debug_snapshot!(run_with_no_config(
        Some("rusty-ferris-club/backpack-e2e-frozen/-/.github/workflows/build.yml"),
        None,
        CopyMode::Apply,
    ));
}

#[test]
#[serial]
fn test_run_source_gist() {
    assert_debug_snapshot!(run_with_no_config(
        Some("https://gist.github.com/jondot/15086f59dab44f30bb10f82ca09f4887"),
        None,
        CopyMode::Apply,
    ));
}
