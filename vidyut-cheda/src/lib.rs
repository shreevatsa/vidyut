#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
#![deny(clippy::unwrap_used)]

pub use crate::chedaka::{Chedaka, Token};
pub use crate::config::Config;
pub use crate::errors::{Error, Result};

mod errors;
mod scoring;

/// Model structs.
///
/// These are exposed for training purposes only.
pub mod model {
    pub use crate::scoring::State;
}

// TODO: move this to its own crate?
pub mod sounds;

// Evaluation code. TODO: move to its own crate?
pub mod conllu;
pub mod dcs;

mod chedaka;
mod config;
mod normalize_text;
mod strict_mode;
