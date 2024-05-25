use beet::prelude::*;
use bevy::prelude::*;
use beet_examples::*;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Default, Component, Serialize, Deserialize)]
struct MyComponent;
#[derive(Debug, Default, Event, Serialize, Deserialize)]
struct MyEvent;
#[derive(Debug, Default, Resource, Serialize, Deserialize)]
struct MyResource;


fn main() {
	let mut app = App::new();
	app.add_plugins(ExampleReplicatePlugin);
	let registry = app.world().resource::<ReplicateRegistry>();
	println!("Replicate Registry:\n{}", registry.types_to_json());
}
