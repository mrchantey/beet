use crate::app_router::app_runner::AppRunner;
use crate::prelude::*;
use axum::Router;
use axum::routing;
use axum::routing::MethodFilter;
use beet_core::prelude::*;
use bevy::prelude::*;
#[cfg(all(debug_assertions, feature = "reload"))]
use tokio::task::JoinHandle;

pub struct AxumRunner {
	pub runner: AppRunner,
}

impl AxumRunner {
	pub fn new(runner: AppRunner) -> Self { Self { runner } }

	#[tokio::main]
	pub async fn run(self, mut app: App) -> Result {
		// take the non-send router to support custom axum routes & layers
		let mut router = app
			.world_mut()
			.remove_non_send_resource::<Router>()
			.unwrap_or_default();

		for route in app
			.world_mut()
			.run_system_cached(BeetRouter::collect_routes)??
		{
			router = instance_to_axum(router, route);
		}

		let html_dir = app
			.world()
			.resource::<WorkspaceConfig>()
			.html_dir
			.into_abs();

		router = router.merge(state_utils_routes());
		// .layer(NormalizePathLayer::trim_trailing_slash());

		match self.runner.mode.unwrap_or_default() {
			RouterMode::Ssg => {
				router =
					router.fallback_service(file_and_error_handler(&html_dir));
			}
			_ => {}
		};

		#[cfg(all(debug_assertions, feature = "reload"))]
		let reload_handle = match self.runner.mode.unwrap_or_default() {
			RouterMode::Ssg => {
				let (reload_layer, reload_handle) = get_reload(html_dir);
				router = router.layer(reload_layer);
				Some(reload_handle)
			}
			_ => None,
		};


		#[cfg(not(debug_assertions))]
		{
			router = router.layer(
				tower_http::trace::TraceLayer::new_for_http()
					.make_span_with(
						tower_http::trace::DefaultMakeSpan::new()
							.level(self.tracing),
					)
					.on_response(
						tower_http::trace::DefaultOnResponse::new()
							.level(self.tracing),
					),
			);
		}

		#[cfg(feature = "lambda")]
		run_lambda(router).await?;
		#[cfg(not(feature = "lambda"))]
		{
			let listener =
				tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
			tracing::info!("\nlistening on http://{}", listener.local_addr()?);
			axum::serve(listener, router).await?;
		}

		#[cfg(all(debug_assertions, feature = "reload"))]
		if let Some(reload_handle) = reload_handle {
			reload_handle.await??;
		}

		Ok(())
	}
}
#[cfg(all(debug_assertions, feature = "reload"))]
fn get_reload(
	html_dir: AbsPathBuf,
) -> (tower_livereload::LiveReloadLayer, JoinHandle<Result<()>>) {
	use beet_utils::prelude::FsWatcher;

	let livereload = tower_livereload::LiveReloadLayer::new();
	let reload = livereload.reloader();

	let reload_handle = tokio::spawn(async move {
		let mut rx = FsWatcher {
			cwd: html_dir.to_path_buf(),
			// debounce: std::time::Duration::from_millis(100),
			// no filter because any change in the html dir should trigger a reload
			..Default::default()
		}
		.watch()?;
		while let Some(ev) = rx.recv().await? {
			if ev.has_mutate() {
				// debug!("html files changed, reloading wasm...");
				reload.reload();
				// println!("{}", events);
				// this2.print_start();
			}
		}
		Ok(())
	});
	(livereload, reload_handle)
}





fn method_to_axum(method: HttpMethod) -> MethodFilter {
	match method {
		HttpMethod::Get => MethodFilter::GET,
		HttpMethod::Post => MethodFilter::POST,
		HttpMethod::Put => MethodFilter::PUT,
		HttpMethod::Patch => MethodFilter::PATCH,
		HttpMethod::Delete => MethodFilter::DELETE,
		HttpMethod::Options => MethodFilter::OPTIONS,
		HttpMethod::Head => MethodFilter::HEAD,
		HttpMethod::Trace => MethodFilter::TRACE,
		HttpMethod::Connect => MethodFilter::CONNECT,
	}
}


fn instance_to_axum(router: Router, instance: RouteInstance) -> Router {
	router.route(
			&instance.route_info.path.to_string_lossy().to_string(),
			routing::on(method_to_axum(instance.route_info.method),
				async move |request: axum::extract::Request| -> HttpResult<axum::response::Response> {
					let beet_request = Request::from_axum(request, &())
						.await
						.map_err(|err| {
							HttpError::bad_request(format!(
								"Failed to extract request: {}",
								err
							))
						})?;

					instance.call(beet_request).await?.into_axum().xok()
				},
			),
		)
}






#[cfg(test)]
mod test {
	use super::*;
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

		let mut router = Router::new();
		for route in world
			.run_system_cached(BeetRouter::collect_routes)
			.unwrap()
			.unwrap()
		{
			router = instance_to_axum(router, route);
		}


		router
			.oneshot_str("/")
			.await
			.unwrap()
			.xpect()
			.to_be("hello world!".to_string());
	}
}
