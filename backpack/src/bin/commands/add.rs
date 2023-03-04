use anyhow::Context;
use anyhow::Result as AnyResult;
use backpack::config::Project;
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
    let repo = subcommand_matches
        .get_one::<String>("repo")
        .map_or_else(|| GitCmd::default().get_local_url(), |r| Ok(r.to_string()))?;

    let (config, _) = Config::load_or_default().context("could not load configuration")?;

    let prompt = &mut Prompt::build(&config, false, None);
    prompt.show_projects(None);
    let name = prompt.ask_for_project_name(&repo)?;

    // add it to the configuration and save
    let mut config = config.clone();
    if let Some(projects) = config.projects.as_mut() {
        projects.insert(name.clone(), Project::from_link(&repo));
    }
    if prompt.are_you_sure(&format!("Save '{name}' ({}) to configuration?", &repo))? {
        config.save()?;
        prompt.say(&format!("Saved '{name}' to global config."));
    }
    prompt.suggest_edit(
        "Would you like to add actions? (will open editor)",
        Config::global_config_file()?.as_path(),
    )?;
    Ok(true)
}
