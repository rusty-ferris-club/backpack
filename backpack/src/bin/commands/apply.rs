use anyhow::Result as AnyResult;
use backpack::{
    data::{CopyMode, Opts},
    run::Runner,
};
use clap::{Arg, ArgMatches, Command};
use core::ops::Deref;

pub fn command() -> Command<'static> {
    Command::new("apply")
        .visible_alias("a")
        .about("apply remote files into a folder")
        .arg(
            Arg::new("git")
                .short('g')
                .long("git")
                .help("clone with git")
                .takes_value(false),
        )
        .arg(
            Arg::new("overwrite")
                .short('w')
                .long("overwrite")
                .help("always overwrite target file(s)")
                .takes_value(false),
        )
        .arg(
            Arg::new("no-cache")
                .short('n')
                .long("no-cache")
                .help("fetch resources without using the cache")
                .takes_value(false),
        )
        .arg(Arg::new("shortlink"))
        .arg(Arg::new("dest"))
}

///
/// apply has a default value for dest ('.'), and will ask user to overwrite
/// interactively upon file conflict
///
pub fn run(_matches: &ArgMatches, subcommand_matches: &ArgMatches) -> AnyResult<bool> {
    let shortlink = subcommand_matches.get_one::<String>("shortlink");
    let dest = subcommand_matches.get_one::<String>("dest");
    let config_file = subcommand_matches
        .get_one::<String>("config")
        .map(String::to_string);

    let r = Runner::default();
    r.run(
        shortlink.map(String::as_str),
        dest.map(String::deref),
        &Opts {
            show_progress: true,
            overwrite: subcommand_matches.is_present("overwrite"),
            is_git: subcommand_matches.is_present("git"),
            no_cache: subcommand_matches.is_present("no-cache"),
            always_yes: false,
            config_file,
            mode: CopyMode::Apply,
        },
    )?;

    Ok(true)
}
