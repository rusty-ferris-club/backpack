use anyhow::Context;
use anyhow::Result as AnyResult;
use backpack::config::Project;
use backpack::data::CopyMode;
use backpack::git::GitCmd;
use backpack::git::GitProvider;
use backpack::{config::Config, ui::Prompt};
use clap::{Arg, ArgMatches, Command};

pub fn command() -> Command<'static> {
    Command::new("add")
        .about("Add a repo as a project")
        .arg(
            Arg::new("git")
                .short('g')
                .long("git")
                .help("prefer a git url")
                .takes_value(false),
        )
        .arg(Arg::new("repo"))
}

pub fn run(_matches: &ArgMatches, subcommand_matches: &ArgMatches) -> AnyResult<bool> {
    // get a repo
    // arg given -> from arg
    // arg not given -> git provider, cmd to extract current remote
    // parse it to see that its valid
    // initialize a Location, and then resolve it per usual.
    // next,
    // -> canonicalize to https and git, and ask which one
    // if exists in config, say it and skip.
    //
    // show current projects,
    // ask what to call it (populate with init-name)
    // with selected, get the current config, load it, mutate, and store
    // done

    // XXX fix this to fallback onto git, and then bail if none there too
    let repo = subcommand_matches
        .get_one::<String>("repo")
        .map_or_else(|| GitCmd::default().get_local_url(), |r| Ok(r.to_string()))?;

    // build Location
    // git preference
    // if user flag -> true
    // otherwise web

    // load configuration
    // show all projects
    // ask how to call the new one
    // store to config
    // ask if to open
    // open with an open crate

    let (config, _) = Config::load_or_default().context("could not load configuration")?;

    let prompt = &mut Prompt::build(&config, false, None);
    prompt.show_projects(&CopyMode::All);
    let name = prompt.ask_for_project_name(&repo)?;

    // add it to the configuration and save
    let mut config = config.clone();
    if let Some(projects) = config.projects.as_mut() {
        projects.insert(name.clone(), Project::from_link(&repo));
    }
    if prompt.are_you_sure(&format!("Save '{}' ({}) to configuration?", name, &repo))? {
        config.save()?;
        prompt.say(&format!("Saved '{}' to global config.", name));
    }
    prompt.suggest_edit(
        "Would you like to add actions? (will open editor)",
        Config::global_config_file()?.as_path(),
    )?;
    Ok(true)
}
