use beet_net::prelude::*;
use bevy::prelude::*;
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
	app.replicate::<MyComponent>()
		.replicate_event::<MyEvent>()
		.replicate_resource::<MyResource>();

	let types = app.world().resource::<ReplicateRegistry>().types_to_json();

	println!("{}", types);
}
