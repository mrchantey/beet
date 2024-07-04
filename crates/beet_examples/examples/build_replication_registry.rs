//! This example is for exporting the example replication registry.
//! Import it into other apps for consistent reg_ids.
use beet::prelude::*;
use beet_examples::prelude::*;
use bevy::prelude::*;


fn main() {
	let mut app = App::new();
	app.add_plugins(ExampleReplicatePlugin);
	let registry = app.world().resource::<ReplicateRegistry>();
	println!("Replicate Registry:\n{}", registry.types_to_json());
}
