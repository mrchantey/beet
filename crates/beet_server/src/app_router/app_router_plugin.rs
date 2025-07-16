use crate::prelude::AppError;
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
pub struct AppRouterPlugin;

/// Runs once before the [`RouteHandler`].
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct BeforeRoute;

/// Runs once after the [`RouteHandler`] and before [`CollectResponse`].
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct AfterRoute;

/// Runs once after [`AfterRoute`] if a [`Response`] is not found, transforming any valid
/// [`RouteHandlerOutput`] into a [`Response`].
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
				handler_output_to_response_plugin::<&'static str>,
				handler_output_to_response_plugin::<String>,
				handler_output_to_response_plugin::<Html>,
				handler_output_to_response_plugin::<Css>,
				handler_output_to_response_plugin::<Javascript>,
				handler_output_to_response_plugin::<Json>,
				handler_output_to_response_plugin::<Png>,
			));
	}
}


fn handler_output_to_response_plugin<
	T: 'static + Send + Sync + IntoResponse,
>(
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
