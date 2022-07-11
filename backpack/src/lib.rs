//#![deny(clippy::pedantic)]
//#![deny(clippy::nursery)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::module_name_repetitions)]

pub mod config;
pub mod content;
pub mod data;
pub mod fetch;
pub mod git;
mod merge;
pub mod run;
pub mod shortlink;
pub mod vendors;
