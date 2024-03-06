#![feature(imported_main)]
pub use sweet::*;
#[path = "./mod.rs"]
mod tests;

extern crate beet_ecs as beet;
