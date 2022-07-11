use anyhow::Result as AnyResult;
use backpack::config::Config;
use clap::{Arg, ArgMatches, Command};
use std::path::Path;

pub fn command() -> Command<'static> {
    Command::new("config")
        .about("create custom configuration")
        .arg(
            Arg::new("init")
                .short('i')
                .long("init")
                .help("initialize an empty configuration file")
                .takes_value(false),
        )
        .arg(
            Arg::new("global")
                .long("global")
                .help("initialize a global configuration")
                .takes_value(false),
        )
        .arg(
            Arg::new("dirs")
                .long("dirs")
                .help("show configuration folders")
                .takes_value(false),
        )
        .arg(
            Arg::new("show")
                .long("show")
                .help("show the merged config (global + local)")
                .takes_value(false),
        )
}

fn print_path(kind: &str, path: &Path) {
    println!(
        "{} {} {}",
        kind,
        if path.exists() { "(found):" } else { "(none):" },
        path.display()
    );
}
pub fn run(_matches: &ArgMatches, subcommand_matches: &ArgMatches) -> AnyResult<bool> {
    if subcommand_matches.is_present("show") {
        let t = Config::load_or_default()?.to_text()?;
        println!("{}", t);
    } else if subcommand_matches.is_present("dirs") {
        let local = Config::local_config_file();
        let global = Config::global_config_file()?;
        print_path("global", global.as_path());
        print_path("local", local.as_path());
    } else if subcommand_matches.is_present("init") {
        let generated = if subcommand_matches.is_present("global") {
            Config::init_global()?
        } else {
            Config::init_local()?
        };
        println!("wrote: {}.", generated.display());
    }

    Ok(true)
}
