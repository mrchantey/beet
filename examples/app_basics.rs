//! For use with scene-based workflows
//!
//! Usage:
//! 1. build scenes: `cargo run -p beet_examples --example build_scenes`
//! 2. run: `cargo run --example app_basics <scene_names>`
//!
//! Common combinations:
//! - hello world: 					`beet-debug camera-2d ui-terminal hello-world`
//! - hello net: 						`beet-debug camera-2d ui-terminal hello-net`
//! - seek: 								`beet-debug camera-2d space-scene seek`
//! - flocking: 						`beet-debug camera-2d space-scene flock`
//! - seek-3d:							`beet-debug ui-terminal lighting-3d ground-3d seek-3d`
//! - animation:						`beet-debug ui-terminal lighting-3d ground-3d hello-animation`
//!
use beet_examples::prelude::*;
use bevy::prelude::*;

fn main() { App::new().add_plugins(ExamplePluginBasics).run(); }
