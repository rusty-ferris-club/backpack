use anyhow::Result as AnyResult;
use backpack::{
    data::CopyMode,
    run::{Opts, Runner},
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
        .arg(
            Arg::new("remote")
                .short('r')
                .long("remote")
                .help("fetch project definitions from a remote config")
                .takes_value(true),
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
    let remote = subcommand_matches
        .get_one::<String>("remote")
        .map(String::to_string);

    let mut r = Runner::default();
    r.show_progress = true;
    r.run(
        shortlink.map(String::as_str),
        dest.map(String::deref),
        &Opts {
            overwrite: subcommand_matches.is_present("overwrite"),
            is_git: subcommand_matches.is_present("git"),
            no_cache: subcommand_matches.is_present("no-cache"),
            no_dest_input: false,
            always_yes: false,
            remote,
            mode: CopyMode::Apply,
        },
    )?;

    Ok(true)
}
