use std::{env, fs};

use anyhow::Result;
use backpack::config::Config;
use backpack::data::{CopyMode, Opts};
use backpack::run::{Runner, RunnerEvents};
use insta::{assert_debug_snapshot, assert_yaml_snapshot};
use requestty_ui::events::KeyCode;
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

fn list_folder(dest: &str) -> Vec<String> {
    let mut r = WalkDir::new(dest)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|file| file.metadata().unwrap().is_file())
        .map(DirEntry::into_path)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect::<Vec<_>>();
    r.sort();
    r
}

fn run(
    shortlink: Option<&str>,
    dest: Option<&str>,
    mode: CopyMode,
    local_config: Option<&str>,
    is_git: bool,
    events: Option<RunnerEvents>,
) -> Result<Vec<String>> {
    let tests_out = "tests-out";
    fs::remove_dir_all(tests_out).ok();
    fs::create_dir(tests_out).unwrap();
    let current_dir = env::current_dir().unwrap();
    env::set_current_dir(tests_out).unwrap();
    if let Some(config) = local_config {
        let _res = fs::remove_file(".backpack.yaml");
        fs::write(".backpack.yaml", config).unwrap();
    }

    if let Some(events) = events {
        Runner::default().run_with_events(
            shortlink,
            dest,
            &Opts {
                show_progress: false,
                overwrite: false,
                is_git,
                no_cache: false,
                always_yes: true,
                remote: None,
                config_file: None,
                mode,
            },
            &events,
        )?;
    } else {
        Runner::default().run(
            shortlink,
            dest,
            &Opts {
                show_progress: false,
                overwrite: false,
                is_git: false,
                no_cache: false,
                always_yes: true,
                remote: None,
                config_file: None,
                mode,
            },
        )?;
    };

    env::set_current_dir(current_dir).unwrap();
    Ok(list_folder(tests_out))
}

fn run_with_no_config(
    shortlink: Option<&str>,
    dest: Option<&str>,
    mode: CopyMode,
    is_git: bool,
    events: Option<RunnerEvents>,
) -> Result<Vec<String>> {
    ensure_no_config();
    run(shortlink, dest, mode, None, is_git, events)
}

fn run_with_local_config(
    shortlink: Option<&str>,
    dest: Option<&str>,
    mode: CopyMode,
    config: &str,
    is_git: bool,
    events: Option<RunnerEvents>,
) -> Result<Vec<String>> {
    ensure_no_config();
    env::remove_var("BP_CONF");
    run(shortlink, dest, mode, Some(config), is_git, events)
}

#[test]
#[serial]
fn test_run_source_dest() {
    assert_debug_snapshot!(run_with_no_config(
        Some("rusty-ferris-club/backpack-e2e-frozen"),
        Some("out"),
        CopyMode::Copy,
        false,
        None,
    ));
}

#[test]
#[serial]
fn test_run_source_dest_subfolder() {
    assert_debug_snapshot!(run_with_no_config(
        Some("rusty-ferris-club/backpack-e2e-frozen/-/.github"),
        Some("out"),
        CopyMode::Copy,
        false,
        None,
    ));
}

#[test]
#[serial]
fn test_run_source_dest_single_file() {
    assert_debug_snapshot!(run_with_no_config(
        Some("rusty-ferris-club/backpack-e2e-frozen/-/.github/workflows/build.yml"),
        Some("out/build.yml"),
        CopyMode::Copy,
        false,
        None,
    ));
}

#[test]
#[serial]
fn test_run_source_single_file() {
    assert_debug_snapshot!(run_with_no_config(
        Some("rusty-ferris-club/backpack-e2e-frozen/-/.github/workflows/build.yml"),
        None,
        CopyMode::Apply,
        false,
        Some(RunnerEvents {
            prompt_events: Some(vec![
                KeyCode::Enter.into(),     // no dest
            ]),
            actions_events: None,
        }),
    ));
}

#[test]
#[serial]
fn test_run_source_gist() {
    assert_debug_snapshot!(run_with_no_config(
        Some("https://gist.github.com/jondot/15086f59dab44f30bb10f82ca09f4887"),
        None,
        CopyMode::Apply,
        false,
        Some(RunnerEvents {
            prompt_events: Some(vec![
                KeyCode::Enter.into(),     // no dest
            ]),
            actions_events: None,
        }),
    ));
}

#[test]
#[serial]
fn test_run_with_local_project_actions() {
    assert_yaml_snapshot!(run_with_no_config(
        Some("rusty-ferris-club/backpack-e2e-frozen-localproj"),
        None,
        CopyMode::Copy,
        false,
        Some(RunnerEvents {
            prompt_events: Some(vec![
                KeyCode::Enter.into(), // default name
            ]),
            actions_events: Some(vec![
                KeyCode::Char('f').into(), // name: 'foo'
                KeyCode::Char('o').into(), //
                KeyCode::Char('o').into(), //
                KeyCode::Enter.into(),     //
            ]),
        }),
    )
    .unwrap());
    assert_yaml_snapshot!(fs::read_to_string("tests-out/my-project1/test.txt").unwrap());
}

#[test]
#[serial]
fn test_run_with_local_project_actions_git_mode() {
    // dont run this in CI, git requires a registered identity
    if env::var("CI").is_err() {
        let res = run_with_no_config(
            Some("rusty-ferris-club/backpack-e2e-frozen-localproj"),
            None,
            CopyMode::Copy,
            true,
            Some(RunnerEvents {
                prompt_events: Some(vec![
                    KeyCode::Enter.into(), // default name
                ]),
                actions_events: Some(vec![
                    KeyCode::Char('f').into(), // name: 'foo'
                    KeyCode::Char('o').into(), //
                    KeyCode::Char('o').into(), //
                    KeyCode::Enter.into(),     //
                ]),
            }),
        )
        .unwrap();
        assert_yaml_snapshot!(res);
        assert_yaml_snapshot!(fs::read_to_string("tests-out/my-project1/test.txt").unwrap());
    }
}

#[test]
#[serial]
fn test_run_with_actions() {
    assert_debug_snapshot!(run_with_local_config(
        Some("integration"),
        None,
        CopyMode::Apply,
        r#"
projects:
  integration:
    shortlink: rusty-ferris-club/backpack-e2e-frozen
    actions:
    - name: "test confirm"
      interaction:
        kind: confirm
        prompt: "are you sure?"
    - name: "grab input and create file"
      interaction:
        kind: input
        prompt: name of your thing
        out: name
      run: touch {{name}}.txt
"#,
        false,
        Some(RunnerEvents {
            actions_events: Some(vec![
                KeyCode::Char('y').into(), // yes
                KeyCode::Enter.into(),     //
                KeyCode::Char('t').into(), // city: 'tlv'
                KeyCode::Char('l').into(), //
                KeyCode::Char('v').into(), //
                KeyCode::Enter.into(),     //
            ]),
            prompt_events: Some(vec![
                KeyCode::Enter.into(),     // no dest
            ]),
        }),
    ));
}

#[test]
#[serial]
fn test_run_with_actions_say_no() {
    assert_debug_snapshot!(run_with_local_config(
        Some("integration"),
        None,
        CopyMode::Apply,
        r#"
projects:
  integration:
    shortlink: rusty-ferris-club/backpack-e2e-frozen
    actions:
    - name: "test confirm"
      interaction:
        kind: confirm
        prompt: "are you sure?"
      break_if_cancel: true
    - name: "grab input and create file"
      interaction:
        kind: input
        prompt: name of your thing
        out: name
      run: touch {{name}}.txt
"#,
        false,
        Some(RunnerEvents {
            actions_events: Some(vec![
                KeyCode::Char('n').into(), // yes
                KeyCode::Enter.into(),     //
            ]),
            prompt_events: Some(vec![
                KeyCode::Enter.into(),     // no dest
            ]),
        }),
    ));
}

#[test]
#[serial]
fn test_run_with_actions_say_yes() {
    assert_debug_snapshot!(run_with_local_config(
        Some("integration"),
        None,
        CopyMode::Apply,
        r#"
projects:
  integration:
    shortlink: rusty-ferris-club/backpack-e2e-frozen
    actions:
    - name: "test confirm"
      interaction:
        kind: confirm
        prompt: "are you sure?"
      break_if_cancel: true
    - name: "grab input and create file"
      interaction:
        kind: input
        prompt: name of your thing
        out: name
      run: touch {{name}}.txt
"#,
        false,
        Some(RunnerEvents {
            actions_events: Some(vec![
                KeyCode::Char('y').into(), // yes
                KeyCode::Enter.into(),     //
                KeyCode::Char('t').into(), // 'tlv'
                KeyCode::Char('l').into(), //
                KeyCode::Char('v').into(), //
                KeyCode::Enter.into(),     //
            ]),
            prompt_events: Some(vec![
                KeyCode::Enter.into(),     // no dest
            ]),
        }),
    ));
}

#[test]
#[serial]
fn test_run_with_actions_say_no_without_break() {
    assert_debug_snapshot!(run_with_local_config(
        Some("integration"),
        None,
        CopyMode::Apply,
        r#"
projects:
  integration:
    shortlink: rusty-ferris-club/backpack-e2e-frozen
    actions:
    - name: "test confirm"
      interaction:
        kind: confirm
        prompt: "are you sure?"
    - name: "grab input and create file"
      interaction:
        kind: input
        prompt: name of your thing
        out: name
      run: touch {{name}}.txt
"#,
        false,
        Some(RunnerEvents {
            actions_events: Some(vec![
                KeyCode::Char('n').into(), // yes
                KeyCode::Enter.into(),     //
                KeyCode::Char('t').into(), // 'tlv'
                KeyCode::Char('l').into(), //
                KeyCode::Char('v').into(), //
                KeyCode::Enter.into(),     //
            ]),
            prompt_events: Some(vec![
                KeyCode::Enter.into(),     // no dest
            ]),
        }),
    ));
}
