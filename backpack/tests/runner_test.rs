use std::path::Path;
use std::{env, fs};

use anyhow::Result;
use backpack::config::Config;
use backpack::data::{CopyMode, Opts};
use backpack::run::{Runner, RunnerEvents};
use insta::assert_debug_snapshot;
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

fn list_folder(dest: &Path) -> Vec<String> {
    let mut r = WalkDir::new(dest)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|file| file.metadata().unwrap().is_file())
        .map(DirEntry::into_path)
        .map(|p| {
            p.to_string_lossy()
                .replace(env::current_dir().unwrap().to_str().unwrap(), "")
                .replace('\\', "/")
        })
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
    // save current dir for restore
    let current_dir = env::current_dir().unwrap();
    let tests_out = current_dir.join("tests-out");
    fs::remove_dir_all(&tests_out).ok();
    fs::create_dir(&tests_out).unwrap();

    // rewire global config to this test-oriented global config folder
    let conf_dir = tests_out.join("backpack-config");
    fs::create_dir(&conf_dir).unwrap();
    if let Some(config) = local_config {
        let _res = fs::remove_file(conf_dir.join("backpack.yaml"));
        fs::write(conf_dir.join("backpack.yaml"), config).unwrap();
    }
    env::set_var("BP_FOLDER", &conf_dir);

    // set up where our actual generated content will be and make bp think it's the current folder
    let content = tests_out.join("content");
    fs::create_dir(&content).unwrap();
    env::set_current_dir(&content).unwrap();

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
                config_file: None,
                mode,
            },
        )?;
    };

    env::set_current_dir(current_dir).unwrap();
    Ok(list_folder(&content))
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
            prompt_events: Some(vec![]),
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
            prompt_events: Some(vec![]),
            actions_events: None,
        }),
    ));
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
            prompt_events: Some(vec![]),
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
            prompt_events: Some(vec![]),
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
            prompt_events: Some(vec![]),
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
            prompt_events: Some(vec![]),
        }),
    ));
}
