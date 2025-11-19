#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

mod bridge;
mod chmlib;
pub mod document;
mod html_to_text;
pub mod parser;
mod update;
mod utils;
mod xml_to_text;

pub use bridge::ffi;
