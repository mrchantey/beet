use beet_core::http_resources::*;
use bevy::prelude::*;

use crate::prelude::AppError;
use crate::prelude::RouteHandlerOutput;


pub struct AppRouterPlugin;


#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct AfterRoute;


impl Plugin for AppRouterPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((
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
		Update,
		(
			output_to_response::<T>
				.run_if(resource_exists::<RouteHandlerOutput<T>>),
			output_to_response::<Result<T, BevyError>>.run_if(
				resource_exists::<RouteHandlerOutput<Result<T, BevyError>>>,
			),
			output_to_response::<Result<T, AppError>>.run_if(
				resource_exists::<RouteHandlerOutput<Result<T, AppError>>>,
			),
		)
			.in_set(AfterRoute),
	);
}


fn output_to_response<T: 'static + Send + Sync + IntoResponse>(
	world: &mut World,
) {
	if let Some(out) = world.remove_resource::<RouteHandlerOutput<T>>() {
		world.insert_resource(out.0.into_response());
	}
}
