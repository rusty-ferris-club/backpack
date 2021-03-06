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
            Arg::new("local")
                .long("local")
                .help("initialize local configuration")
                .takes_value(false),
        )
        .arg(
            Arg::new("sync")
                .short('s')
                .long("sync")
                .help("synchronize remote config sources")
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
    if subcommand_matches.is_present("sync") {
        let (c, _) = Config::load_or_default()?;
        c.sync(|s| {
            println!("downloading: {}", s.name);
        })?;
        println!("done.");
    } else if subcommand_matches.is_present("init") {
        let generated = if subcommand_matches.is_present("local") {
            Config::init_local()?
        } else {
            Config::init_global()?
        };
        println!("wrote: {}.", generated.display());
    } else {
        let local = Config::local_config_file();
        let global = Config::global_config_file()?;
        print_path("global", global.as_path());
        print_path("local", local.as_path());

        let t = Config::load_or_default()?.0.to_text()?;
        println!("{}", t);
    }

    Ok(true)
}
