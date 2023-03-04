use std::path::Path;

use anyhow::Context;
use anyhow::Result as AnyResult;
use backpack::config::LocalProjectConfig;
use backpack::config::Project;
use backpack::config::PROJECT_CONFIG_FILE;
use backpack::git::GitCmd;
use backpack::git::GitProvider;
use backpack::{config::Config, ui::Prompt};
use clap::{Arg, ArgMatches, Command};

pub fn command() -> Command<'static> {
    Command::new("add")
        .about("Save a repo as a project")
        .arg(
            Arg::new("git")
                .short('g')
                .long("git")
                .help("Prefer a git url")
                .takes_value(false),
        )
        .arg(Arg::new("repo"))
}

pub fn run(_matches: &ArgMatches, subcommand_matches: &ArgMatches) -> AnyResult<bool> {
    let config = Config::load_or_default().context("could not load configuration")?;
    let prompt = &mut Prompt::build(&config, false, None);

    prompt.show_projects();

    let local_project = LocalProjectConfig::from_path(&Path::new(".").join(PROJECT_CONFIG_FILE))?;
    let (name, repo, new_config) = if let Some(local_project) = local_project {
        prompt.say("Found local project config. Reading actions and swaps from it.");
        let name = prompt.ask_for_project_name(&local_project.project.shortlink)?;
        let mut config = config.clone();
        if let Some(projects) = config.projects.as_mut() {
            projects.insert(name.clone(), local_project.project.clone());
        }
        (name, local_project.project.shortlink.clone(), config)
    } else {
        let repo = subcommand_matches
            .get_one::<String>("repo")
            .map_or_else(|| GitCmd::default().get_local_url(), |r| Ok(r.to_string()))?;

        let name = prompt.ask_for_project_name(&repo)?;
        // add it to the configuration and save
        let mut config = config.clone();
        if let Some(projects) = config.projects.as_mut() {
            projects.insert(name.clone(), Project::from_link(&repo));
        }
        (name, repo, config)
    };

    // save the new, modified, copy of config
    if prompt.are_you_sure(&format!("Save '{name}' ({repo}) to configuration?"))? {
        new_config.save()?;
        prompt.say(&format!("Saved '{name}' to global config."));
    }

    prompt.suggest_edit(
        "Would you like to add actions? (will open editor)",
        Config::global_config_file()?.as_path(),
    )?;
    Ok(true)
}
