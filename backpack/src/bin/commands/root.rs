//use anyhow::Result as AnyResult;
use clap::crate_version;
use clap::Command;

pub fn command() -> Command<'static> {
    Command::new("backpack")
        .version(crate_version!())
        .subcommand_required(true)
        .arg_required_else_help(true)
        .about("Create projects from existing repos")
}

/*
pub fn run(_matches: &ArgMatches) -> AnyResult<bool> {
    Ok(true)
}
*/
