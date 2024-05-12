// use beet::prelude::*;
use bevy::prelude::*;
// use example_plugin::ExamplePlugin;
#[path = "common/common.rs"]
mod common;
use common::*;

fn main() {
	App::new()
		.add_plugins(ExamplePlugin3d)
		.add_plugins(DialogPanelPlugin)
		.run();
}