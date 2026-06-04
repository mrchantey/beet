//! The `beet` CLI: build, serve, and run-wasm helpers for beet apps.
//!
//! Every command is an [`Action`](beet::prelude::Action) served as a route on a
//! [`CliServer`](beet::prelude::CliServer)-backed [`router`](beet::prelude::router),
//! so `beet --help` lists them and `beet <command>` dispatches. The entry point
//! and route wiring live in `main.rs`.

mod commands;

pub mod prelude {
	pub use crate::commands::*;
}
