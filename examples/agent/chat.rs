//! This example demonstrates a cli chat workflow,
//! see [`CliAgentPlugin`] for more options.
use beet::prelude::*;
use clap::Parser;

pub fn main() {
	let mut plugin = CliAgentPlugin::parse();

	// enabling this will add the image generation tool to the agent
	// plugin.generate_images = true;

	if !plugin.initial_message() {
		plugin.initial_prompt = Some(
			"make a case for bevy becoming the platform of choice for ambitious applications of the future in 100 words".into(),
		);
	}

	App::new()
		.add_plugins((MinimalPlugins, plugin))
		.run()
		.into_result()
		.unwrap();
}
