use std::sync::Arc;

// #![allow(unused)]
use axum::extract::Json;
use axum::extract::Query as QueryParams;
use beet_common::prelude::*;
use beet_rsx::prelude::*;
use bevy::ecs::schedule::Schedulable;
use bevy::ecs::schedule::ScheduleConfigs;
use bevy::ecs::system::BoxedSystem;
use bevy::ecs::system::RunSystemOnce;
use bevy::ecs::system::ScheduleSystem;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use serde::Deserialize;
use tokio::sync::RwLock;


fn foo(_app: &mut App) {}

/// A server app can be used for managing global state like a database connection pool.
type ServerApp = Arc<RwLock<App>>;
// each route gets its own lightweight world, populated by its http system.
type RouteWorld = World;


// An example request payload, this could be any json or query param payload
#[derive(Deserialize)]
struct RequestPayload {
	name: String,
}

// a http system is named after its method ie `get`,
// and the route is the file path, ie `src/pages/contact-us.rs`
fn get(
	// System Input, if any, is a tuple of axum extractors
	payload: In<QueryParams<RequestPayload>>,
	// otherwise its a regular system
	res: Res<Time>,
) -> impl Bundle {
	let name = payload.name.clone();
	rsx! {
		<body>
			<h1>hello {name}!</h1>
			<p>time: {res.elapsed_secs()}</p>
		</body>
	}
}

// a small amout of codegen to build the actual route for axum
fn route(world: &mut World, val: MyExtractor) {
	let bundle = world.run_system_once_with(get, val).unwrap();
	world.spawn(bundle);
	// async world is part of the axum state
}


// fn bar(_world: &mut World) -> impl Bundle { () }


/// The returned type of the blanket [`IntoScheduleConfigs::into_configs`].
///
/// ## Example
///
/// ```rust
/// # use bevy::prelude::*;
/// fn my_bevy_system(){}
/// let any_system = my_bevy_system.into_configs();
/// ```
type AnySystem = ScheduleConfigs<ScheduleSystem>;
type Routes = HashMap<String, AnySystem>;


fn insert<M>(
	routes: &mut Routes,
	path: &str,
	method: impl IntoScheduleConfigs<ScheduleSystem, M>,
) {
	routes.insert(path.to_string(), method.into_configs());
}

fn main() {
	let mut app = App::new();
	// app.add_systems(schedule, systems)

	let mut routes = Routes::default();
	insert(&mut routes, "/contact-us", get);
}
