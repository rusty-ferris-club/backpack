//#![deny(clippy::pedantic)]
//#![deny(clippy::nursery)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::use_self)]
#![allow(clippy::unused_self)]
#![feature(path_try_exists)]

pub mod config;
pub mod content;
pub mod data;
pub mod fetch;
pub mod git;
mod merge;
pub mod prompt;
pub mod run;
pub mod shortlink;
pub mod vendors;
