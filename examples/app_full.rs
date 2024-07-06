//! For use with scene-based workflows
//!
//! Usage:
//! 1. build scenes: `cargo run -p beet_examples --example build_scenes`
//! 2. run: `cargo run --example app_full <scene_names>`
//!
//! Common combinations:
//! - hello llm: 						`beet-debug camera-2d ui-terminal sentence-selector`
//! - fetch:								`ui-terminal-input lighting-3d ground-3d fetch-scene fetch-npc`
//! - frozen lake - train:	`beet-debug ui-terminal lighting-3d frozen-lake-scene frozen-lake-train`
//! - frozen lake - run:		`beet-debug ui-terminal lighting-3d frozen-lake-scene frozen-lake-run`
//!
use beet_examples::prelude::*;
use bevy::prelude::*;

fn main() { App::new().add_plugins(ExamplePluginFull).run(); }
