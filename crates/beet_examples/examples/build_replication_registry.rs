//! This example is for exporting the example replication registry.
//! Import it into other apps for consistent reg_ids.
use anyhow::Result;
use beet::prelude::*;
use beet_examples::prelude::*;
use bevy::prelude::*;
use std::fs;

fn main() -> Result<()> {
	let mut app = App::new();
	app.add_plugins(ExampleReplicatePlugin);
	let registry = app.world().resource::<ReplicateRegistry>();
	let json = registry.types_to_json();
	let path = "target/replication_registry.json";
	fs::write(path, json)?;
	println!("Wrote Replication registry:\nPath: {path}\nDetails:\n{}", registry.types_to_json());
	Ok(())
}
