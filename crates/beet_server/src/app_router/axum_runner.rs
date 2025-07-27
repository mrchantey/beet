use crate::app_router::app_runner::AppRunner;
use crate::prelude::*;
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


	pub fn from_world(world: &mut World, router: axum::Router) -> axum::Router {
		let clone_world = CloneWorld::new(world);
		// Add a catch-all fallback handler for unmatched routes
		let route = routing::any(move |request: axum::extract::Request| {
			let world = clone_world.clone();
			async move {
				handle_axum_request(world, request)
					.await
					.into_response()
					.into_axum().await
			}
		});

		router
			.route("/", route.clone())
			.route("/{*wildcard}", route)
	}

	#[tokio::main]
	pub async fn run(self, mut app: App) -> Result {
		// take the non-send router to support custom axum routes & layers
		let mut router = app
			.world_mut()
			.remove_non_send_resource::<axum::Router>()
			.unwrap_or_default();

		router = Self::from_world(app.world_mut(), router);

		router = router.merge(state_utils_routes());
		// .layer(NormalizePathLayer::trim_trailing_slash());
		let html_dir = app
			.world()
			.resource::<WorkspaceConfig>()
			.html_dir
			.into_abs();

		match self.runner.mode.unwrap_or_default() {
			RouterMode::Ssg => {
				debug!(
					"Serving static files from:\n{}",
					&html_dir
				);
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

#[allow(unused)]
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


async fn handle_axum_request(
	clone_world: CloneWorld,
	request: axum::extract::Request,
) -> Result<Response> {
	let world = clone_world.clone_world()?;

	let request = Request::from_axum(request, &()).await?;

	let (_world, response) = Router::handle_request(world, request).await;

	Ok(response)
}






#[cfg(test)]
mod test {
	use super::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn works() {
		let mut app = App::new();
		app.add_plugins(RouterPlugin);
		app.world_mut().spawn((
			RouteFilter::new("pizza"),
			RouteHandler::new(|| "hello world!"),
		));

		// these tests also test the roundtrip CloneWorld mechanism
		// catching errors like missing app.register_type::<T>()
		AxumRunner::from_world(app.world_mut(), axum::Router::new())
			.oneshot_res("/dsfkdsl")
			.await
			.unwrap()
			.status()
			.xpect()
			.to_be(StatusCode::NOT_FOUND);
		AxumRunner::from_world(app.world_mut(), axum::Router::new())
			.oneshot_str("/pizza")
			.await
			.unwrap()
			.xpect()
			.to_be("hello world!".to_string());
	}
}
