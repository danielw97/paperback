#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

mod bridge;
mod document;
mod parser;
mod update;
mod utils;

pub use bridge::ffi;
