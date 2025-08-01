use crate::app_router::server_runner::ServerRunner;
use crate::prelude::*;
use axum::routing;
use axum::routing::MethodFilter;
use beet_core::prelude::*;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
#[cfg(all(debug_assertions, feature = "reload"))]
use tokio::task::JoinHandle;

// use tower::Layer;
// use tower_http::normalize_path::NormalizePath;
// use tower_http::normalize_path::NormalizePathLayer;

pub struct AxumRunner {
	pub runner: ServerRunner,
}

impl AxumRunner {
	pub fn new(runner: ServerRunner) -> Self { Self { runner } }


	/// Create a new [`axum::Router`] using the current app's world.
	/// All handlers will get the world from the [`AppPool`]
	pub fn router(world: &mut World) -> axum::Router {
		let mut router = world
			.get_non_send_resource::<axum::Router>()
			.cloned()
			.unwrap_or_default();
		let pool = world.resource::<Router>().clone();

		let handler_pool = pool.clone();
		let handler = move |request: axum::extract::Request| {
			// let pool = pool.clone();
			async move {
				// let pool = pool.clone();
				match async move {
					// let world = world.clone_world()?;
					let req = Request::from_axum(request, &()).await?;
					let mut app = handler_pool.get();
					let world = app.world_mut();
					Router::handle_request(world, req).await.xok::<BevyError>()
				}
				.await
				{
					Ok(res) => res.into_axum().await,
					Err(err) => err.into_response().into_axum().await,
				}
			}
		};

		let app_pool_endpoints = pool
			.get()
			.world_mut()
			.run_system_once(ResolvedEndpoint::collect)
			.unwrap();

		for (_, endpoint) in app_pool_endpoints {
			let segments = segments_to_axum(endpoint.segments().clone());
			let method = method_to_axum(endpoint.method());
			trace!("Registering endpoint: {} {}", endpoint.method(), &segments);
			router =
				router.route(&segments, routing::on(method, handler.clone()));
		}
		router
	}

	#[tokio::main]
	pub async fn run(self, mut app: App) -> Result {
		let mut router = Self::router(app.world_mut());

		router = router.merge(state_utils_routes());
		// .layer(NormalizePathLayer::trim_trailing_slash());
		let html_dir = app
			.world()
			.resource::<WorkspaceConfig>()
			.html_dir
			.into_abs();

		match self.runner.mode.unwrap_or_default() {
			RouterMode::Ssg => {
				debug!("Serving static files from:\n{}", &html_dir);
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

/// Convert a vector of RouteSegment to a string representation for axum routing
fn segments_to_axum(segments: Vec<RouteSegment>) -> String {
	let path = segments
		.into_iter()
		.map(|segment| match segment {
			RouteSegment::Static(seg) => seg,
			RouteSegment::Dynamic(seg) => format!("{{{seg}}}"),
			RouteSegment::Wildcard(seg) => format!("{{*{seg}}}"),
		})
		.collect::<Vec<_>>()
		.join("/");
	format!("/{}", path)
}



#[cfg(test)]
mod test {
	use super::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn works() {
		let mut app = App::new();
		app.add_plugins(RouterPlugin).insert_resource(Router::new(
			|app: &mut App| {
				app.world_mut().spawn((
					RouteFilter::new("pizza"),
					RouteHandler::new(HttpMethod::Get, || "hello world!"),
				));
			},
		));

		// these tests also test the roundtrip CloneWorld mechanism
		// catching errors like missing app.register_type::<T>()
		AxumRunner::router(app.world_mut())
			.oneshot_res("/dsfkdsl")
			.await
			.unwrap()
			.status()
			.xpect()
			.to_be(StatusCode::NOT_FOUND);
		AxumRunner::router(app.world_mut())
			.oneshot_str("/pizza")
			.await
			.unwrap()
			.xpect()
			.to_be("hello world!".to_string());
	}
}
