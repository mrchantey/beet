use crate::prelude::*;
use beet_core::prelude::*;
use std::future::Future;

impl<S: 'static + Send + Sync + Clone> AddRoute for axum::Router<S> {
	fn add_route<Fut>(
		self,
		route_info: &RouteInfo,
		func: impl 'static + Send + Sync + Clone + Fn(Request) -> Fut,
	) -> Self
	where
		Fut: Future<Output = AppResult<Response>> + Send,
		Self: Sized,
	{
		use axum::routing;

		let router = self.route(
			&route_info.path.to_string_lossy(),
			routing::on(
				route_info.method.into_axum_method(),
				async move |request: axum::extract::Request| -> AppResult<axum::response::Response> {
					let beet_request = Request::from_axum(request, &())
						.await
						.map_err(|err| {
							AppError::bad_request(format!(
								"Failed to extract request: {}",
								err
							))
						})?;

					func(beet_request).await?.into_axum().xok()
				},
			),
		);

		router
	}
}

impl axum::response::IntoResponse for AppError {
	fn into_response(self) -> axum::response::Response {
		(self.status_code, self.message).into_response()
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn works() {
		let mut world = World::new();
		world.spawn(children![(
			RouteInfo::get("/"),
			RouteHandler::new(|mut commands: Commands| {
				commands.insert_resource("hello world!".into_response());
			})
		),]);

		world
			.run_system_cached_with(collect_routes, Router::default())
			.unwrap()
			.unwrap()
			.with_state::<()>(AppRouterState::default())
			.oneshot_str("/")
			.await
			.unwrap()
			.xpect()
			.to_be("hello world!".to_string());
	}
}
