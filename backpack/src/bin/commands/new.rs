use anyhow::Result as AnyResult;
use backpack::{
    data::CopyMode,
    run::{Opts, Runner},
};
use clap::{Arg, ArgMatches, Command};

pub fn command() -> Command<'static> {
    Command::new("new")
        .visible_alias("n")
        .about("initialize a new project")
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
        .arg(Arg::new("shortlink").required(true))
        .arg(Arg::new("name").required(true))
}

///
/// new will only create a new folder with contents, no overwriting and no
/// default dest folder
///
pub fn run(_matches: &ArgMatches, subcommand_matches: &ArgMatches) -> AnyResult<bool> {
    let shortlink = subcommand_matches.get_one::<String>("shortlink").unwrap();
    let dest = subcommand_matches.get_one::<String>("name").unwrap();
    let mut r = Runner::default();
    r.show_progress = true;
    r.run(
        shortlink,
        dest,
        &Opts {
            overwrite: false,
            is_git: subcommand_matches.is_present("git"),
            no_cache: subcommand_matches.is_present("no-cache"),
            mode: CopyMode::Copy,
        },
    )?;

    Ok(true)
}
