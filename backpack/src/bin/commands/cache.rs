use anyhow::Context;
use anyhow::Result as AnyResult;
use backpack::config::Config;
use clap::{Arg, ArgMatches, Command};
use std::fs;

pub fn command() -> Command<'static> {
    Command::new("cache")
        .about("cache handling")
        .arg(
            Arg::new("rm")
                .long("rm")
                .help("remove the cache")
                .takes_value(false),
        )
        .arg(
            Arg::new("path")
                .long("path")
                .help("show where the cache is stored")
                .takes_value(false),
        )
}

///
/// new will only create a new folder with contents, no overwriting and no
/// default dest folder
///
pub fn run(_matches: &ArgMatches, subcommand_matches: &ArgMatches) -> AnyResult<bool> {
    if subcommand_matches.is_present("rm") {
        fs::remove_dir_all(Config::global_cache_folder()?)
            .context("cannot remove cache (maybe there's no cache yet?)")?;
        println!("cache removed.");
    }

    if subcommand_matches.is_present("path") {
        println!("{}", Config::global_cache_folder()?.display());
    }
    Ok(true)
}
