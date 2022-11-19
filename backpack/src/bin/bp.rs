mod commands;
use std::process::exit;
use tracing_subscriber::{filter, EnvFilter, Registry};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() {
    Registry::default()
        .with(tracing_forest::ForestLayer::default())
        .with(
            EnvFilter::builder()
                .with_default_directive(filter::LevelFilter::OFF.into())
                .with_env_var("LOG")
                .from_env_lossy(),
        )
        .init();

    let app = commands::root::command()
        .subcommand(commands::cache::command())
        .subcommand(commands::add::command())
        .subcommand(commands::config::command());

    let matches = app.clone().get_matches();

    let res = match matches.subcommand() {
        Some(tup) => match tup {
            ("cache", subcommand_matches) => commands::cache::run(&matches, subcommand_matches),
            ("add", subcommand_matches) => commands::add::run(&matches, subcommand_matches),
            ("config", subcommand_matches) => commands::config::run(&matches, subcommand_matches),
            (maybe_shortlink, _) => {
                unreachable!("unexpected subcommand: {}", maybe_shortlink);
            }
        },
        _ => commands::root::run(&matches),
    };

    match res {
        Ok(ok) => {
            exit(if ok { 0 } else { 1 });
        }
        Err(err) => {
            eprintln!("error: {}", err);
            exit(1)
        }
    }
}
