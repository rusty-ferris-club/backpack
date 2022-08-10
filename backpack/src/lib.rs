//#![deny(clippy::pedantic)]
//#![deny(clippy::nursery)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::use_self)]
#![allow(clippy::unused_self)]
#![allow(clippy::missing_const_for_fn)]

pub mod config;
pub mod content;
pub mod data;
pub mod fetch;
pub mod git;
pub mod run;
pub mod shortlink;
pub mod ui;
pub mod vendors;
