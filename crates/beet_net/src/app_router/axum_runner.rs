use crate::prelude::*;
use axum::routing;
use axum::routing::MethodFilter;
use beet_core::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
#[cfg(all(debug_assertions, feature = "reload"))]
use tokio::task::JoinHandle;

// use tower::Layer;
// use tower_http::normalize_path::NormalizePath;
// use tower_http::normalize_path::NormalizePathLayer;

pub struct AxumRunner {
	// pub runner: ServerRunner,
}

impl AxumRunner {
	pub fn new() -> Self { Self {} }


	/// Create a new [`axum::Router`] using the current app's world.
	/// All handlers will get the world from the [`AppPool`]
	pub fn router(world: &mut World) -> axum::Router {
		let mut axum_router = world
			.get_non_send_resource::<axum::Router>()
			.cloned()
			.unwrap_or_default();
		let beet_router = world.resource::<Router>().clone();

		let handler = move |axum_req: axum::extract::Request| async move {
			match async move {
				let beet_req = Request::from_axum(axum_req, &()).await?;
				beet_router
					.handle_request(beet_req)
					.await
					.xok::<BevyError>()
			}
			.await
			{
				Ok(beet_res) => beet_res.into_axum().await,
				Err(bevy_err) => bevy_err.into_response().into_axum().await,
			}
		};

		trace!("Registering catch-all endpoint");
		axum_router = axum_router.route("/", routing::any(handler.clone()));
		axum_router = axum_router.route("/{*any_path}", routing::any(handler));
		// let router_mode = world.resource::<RouterMode>().clone();
		// for (_, endpoint) in beet_router
		// 	.pop()
		// 	.run_system_once(ResolvedEndpoint::collect)
		// 	.unwrap()
		// 	.into_iter()
		// 	.filter(|(_, info)| {
		// 		// only register non-static endpoints in ssg
		// 		if matches!(router_mode, RouterMode::Ssg) {
		// 			!info.is_static_html()
		// 		} else {
		// 			true
		// 		}
		// 	}) {
		// 	let segments = segments_to_axum(endpoint.segments().clone());
		// 	let method = method_to_axum(endpoint.method());
		// 	trace!("Registering endpoint: {} {}", endpoint.method(), &segments);
		// 	axum_router = axum_router
		// 		.route(&segments, routing::on(method, handler.clone()));
		// }
		axum_router
	}

	pub async fn run(self, world: &mut World) -> Result {
		let mut router = Self::router(world);

		router = router.merge(state_utils_routes());

		#[cfg(all(debug_assertions, feature = "reload"))]
		let reload_handle = {
			let html_dir =
				world.resource::<WorkspaceConfig>().html_dir.into_abs();
			let (reload_layer, reload_handle) = get_reload(html_dir);
			router = router.layer(reload_layer);
			Some(reload_handle)
		};


		#[cfg(any(not(debug_assertions), feature = "lambda"))]
		{
			router = router.layer(
				tower_http::trace::TraceLayer::new_for_http()
					.make_span_with(
						tower_http::trace::DefaultMakeSpan::new(), // .level(self.runner.tracing),
					)
					.on_response(
						tower_http::trace::DefaultOnResponse::new(), // .level(self.runner.tracing),
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

/// Convert a vector of [`PathSegment`] to a string representation for axum routing
/// using axum >0.8 syntax.
#[allow(unused)]
fn segments_to_axum(segments: Vec<PathSegment>) -> String {
	let path = segments
		.into_iter()
		.map(|segment| match segment {
			PathSegment::Static(seg) => seg,
			PathSegment::Dynamic(seg) => format!("{{{seg}}}"),
			PathSegment::Wildcard(seg) => format!("{{*{seg}}}"),
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
		app.add_plugins(RouterPlugin)
			.insert_resource(Router::new_bundle(|| {
				(
					PathFilter::new("pizza"),
					RouteHandler::endpoint(|| "hello world!"),
				)
			}));

		// these tests also test the roundtrip CloneWorld mechanism
		// catching errors like missing app.register_type::<T>()
		AxumRunner::router(app.world_mut())
			.oneshot_res("/dsfkdsl")
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::NOT_FOUND);
		AxumRunner::router(app.world_mut())
			.oneshot_str("/pizza")
			.await
			.unwrap()
			.xpect_eq("hello world!".to_string());
	}
}
