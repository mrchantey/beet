#![allow(unused)]
use axum::extract::FromRequest;
use axum::extract::FromRequestParts;
use axum::extract::Request;
use axum::handler::Handler;
use axum::response::IntoResponse;
use axum::routing::MethodRouter;
use beet_template::prelude::*;
use bevy::ecs::schedule::ScheduleConfigs;
use bevy::ecs::system::ScheduleSystem;
use http::StatusCode;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;
// use bevy::platform::collections::HashMap;
use crate::prelude::*;
use bevy::prelude::*;
use http::request::Parts;




// /// A level of indirection applied as [`ScheduleConfigs`] are not `Send` or `Sync`
// type RouteHandler =
// 	Box<dyn 'static + Send + Sync + Fn() -> ScheduleConfigs<ScheduleSystem>>;
// fn wrap_route<E: FromRequest<AppState, M>, M>(
// 	mut world: World,
// 	extractors: E,
// ) -> impl IntoResponse {
// }


#[derive(Debug, Default, Clone)]
pub struct AppState {}




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


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use axum::Router;
	use axum::extract::Query as QueryParams;
	use axum::routing::get;
	use beet_common::prelude::*;
	use beet_template::prelude::*;
	use bevy::prelude::*;
	use serde::Deserialize;
	// use sweet::prelude::*;

	// An example request payload, this could be any json or query param
	#[derive(Deserialize)]
	struct RequestPayload {
		name: String,
	}
	fn my_route(
		// System Input, if any, is a tuple of axum extractors
		payload: QueryParams<RequestPayload>,
		// otherwise its a regular system
		// query: Query<&Name>,
		// world: &mut World,
	) -> impl Bundle {
		let name = payload.name.clone();
		rsx! {
			<body>
				<h1>hello {name}!</h1>
				// <p>time: {time.elapsed_secs()}</p>
			</body>
		}
	}

	#[test]
	fn works() {
		let _router: Router<AppState> = Router::new()
			.route("/test", get(|| async { "Hello, World!" }))
			.with_state(AppState::default());
	}
}
