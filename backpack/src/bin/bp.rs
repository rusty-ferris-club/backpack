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
        .subcommand(commands::new::command())
        .subcommand(commands::apply::command())
        .subcommand(commands::cache::command())
        .subcommand(commands::config::command());

    let matches = app.clone().get_matches();

    let res = match matches.subcommand() {
        Some(tup) => match tup {
            ("new", subcommand_matches) => commands::new::run(&matches, subcommand_matches),
            ("apply", subcommand_matches) => commands::apply::run(&matches, subcommand_matches),
            ("cache", subcommand_matches) => commands::cache::run(&matches, subcommand_matches),
            ("config", subcommand_matches) => commands::config::run(&matches, subcommand_matches),
            _ => unreachable!(),
        },
        _ => unreachable!(),
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
