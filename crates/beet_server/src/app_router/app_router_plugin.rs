//! Plugin for the Beet router lifecycle
//!
//!
//!
use crate::prelude::AppError;
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
pub struct AppRouterPlugin;

/// The main schedule for layers that run before the [`RouteHandler`],
/// like authentication.
/// ## Before:
/// - [`BeforeRoute`]
/// - [`RouteHandler`]
/// - [`CollectResponse`]
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct BeforeRoute;

/// The main schedule for layers that handle the [`RouteHandlerOutput`],
/// usually to convert it into a [`Response`].
/// ## After
/// - [`BeforeRoute`]
/// - [`RouteHandler`]
/// ## Before:
/// - [`CollectResponse`]
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct AfterRoute;

/// The final lifecycle schedule, transforming any valid [`RouteHandlerOutput`]
/// into a [`Response`] if no response is present.
/// ## After
/// - [`BeforeRoute`]
/// - [`RouteHandler`]
/// - [`AfterRoute`]
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct CollectResponse;

impl Plugin for AppRouterPlugin {
	fn build(&self, app: &mut App) {
		app
			// dont initialize empty, faster?
			// .init_schedule(BeforeRoute)
			// .init_schedule(AfterRoute)
			// .init_schedule(CollectResponse)
			.add_plugins((
				// this should match all IntoResponse types in http_resources.rs
				handler_output_plugin::<&'static str>,
				handler_output_plugin::<String>,
				handler_output_plugin::<Html>,
				handler_output_plugin::<Css>,
				handler_output_plugin::<Javascript>,
				handler_output_plugin::<Json>,
				handler_output_plugin::<Png>,
			));
	}
}

/// Converts
fn handler_output_plugin<T: 'static + Send + Sync + IntoResponse>(
	app: &mut App,
) {
	app.add_systems(
		CollectResponse,
		(
			output_to_response::<T>
				.run_if(resource_exists::<RouteHandlerOutput<T>>),
			output_to_response::<Result<T, BevyError>>.run_if(
				resource_exists::<RouteHandlerOutput<Result<T, BevyError>>>,
			),
			output_to_response::<Result<T, AppError>>.run_if(
				resource_exists::<RouteHandlerOutput<Result<T, AppError>>>,
			),
			bundle_layer
				.run_if(resource_exists::<RouteHandlerOutput<BoxedBundle>>),
			documents_to_response,
		),
	);
}

fn output_to_response<T: 'static + Send + Sync + IntoResponse>(
	world: &mut World,
) {
	if let Some(out) = world.remove_resource::<RouteHandlerOutput<T>>() {
		world.insert_resource(out.0.into_response());
	}
}
