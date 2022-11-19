use anyhow::Result;
use backpack::data::CopyMode;
use backpack::data::Opts;
use backpack::run::Runner;
use clap::Arg;
use clap::ArgMatches;
//use anyhow::Result as AnyResult;
use clap::crate_version;
use clap::Command;

pub fn command() -> Command<'static> {
    Command::new("backpack")
        .version(crate_version!())
        .subcommand_required(false)
        .arg_required_else_help(false)
        .about("Set up projects and download files from existing repos")
        .arg(
            Arg::new("git")
                .short('g')
                .long("git")
                .help("Clone with git")
                .takes_value(false),
        )
        .arg(
            Arg::new("overwrite")
                .short('w')
                .long("overwrite")
                .help("Always overwrite target file(s)")
                .takes_value(false),
        )
        .arg(
            Arg::new("fetch")
                .short('f')
                .long("fetch")
                .help("Fetch and apply into the current folder")
                .takes_value(false),
        )
        .arg(
            Arg::new("no-cache")
                .short('n')
                .long("no-cache")
                .help("Fetch resources without using the cache")
                .takes_value(false),
        )
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .help("Use a specified configuration file")
                .takes_value(true),
        )
        .arg(clap::Arg::new("shortlink").help("A full or a shortlink to a repo (e.g. org/user)"))
        .arg(Arg::new("dest").help("Target folder"))
}

pub fn run(matches: &ArgMatches) -> Result<bool> {
    let shortlink = matches.get_one::<String>("shortlink");
    let dest = matches.get_one::<String>("dest");
    let config_file = matches.get_one::<String>("config").map(String::to_string);
    let mode = if matches.is_present("fetch") {
        CopyMode::Apply
    } else {
        CopyMode::Copy
    };
    let r = Runner::default();
    r.run(
        shortlink.map(String::as_str),
        dest.map(String::as_str),
        &Opts {
            show_progress: true,
            overwrite: matches.is_present("overwrite"),
            is_git: matches.is_present("git"),
            no_cache: matches.is_present("no-cache"),
            always_yes: false,
            config_file,
            mode,
        },
    )?;

    Ok(true)
}
