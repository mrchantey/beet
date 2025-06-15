use crate::prelude::*;
use anyhow::Result;
use axum::Router;
#[cfg(not(feature = "lambda"))]
use axum::ServiceExt;
#[cfg(not(feature = "lambda"))]
use axum::body::Body;
use beet_net::prelude::*;
// use beet_router::types::RouteFunc;
#[cfg(feature = "lambda")]
use lambda_http::Body;
use std::path::PathBuf;
#[cfg(all(debug_assertions, feature = "reload"))]
use tokio::task::JoinHandle;
use tower::Service;
// use tower_http::trace::TraceLayer;
// use tower_http::trace;
// use tracing::Level;

// use tower::Layer;
// use tower_http::normalize_path::NormalizePath;
// use tower_http::normalize_path::NormalizePathLayer;


/// An Axum Server with file based routing and a live reload dev server.
pub struct BeetServer<S> {
	pub html_dir: PathBuf,
	pub router: Router<S>,
}

impl Default for BeetServer<()> {
	fn default() -> Self {
		Self {
			html_dir: "target".into(),
			router: Router::default(),
		}
	}
}

impl<S> BeetServer<S> {
	pub fn bundle_route<M>(
		mut self,
		info: RouteInfo,
		route: impl BundleRoute<M, State = S>,
	) -> Self
	where
		S: 'static + Send + Sync + Clone,
	{
		self.router = self.router.route(
			&info.path.to_string(),
			route.into_method_router(info.method),
		);
		self
	}

	/// Server the provided router, adding
	/// a fallback file server with live reload.
	pub async fn serve(self) -> Result<()>
	where
		S: Clone + Send + Sync + 'static,
		Router<S>: Service<
				http::Request<Body>,
				Response = axum::response::Response,
				Error = std::convert::Infallible,
			> + Clone,
		<Router<S> as Service<http::Request<Body>>>::Future: Send,
	{
		#[allow(unused_mut)]
		let mut router = self
			.router
			.fallback_service(file_and_error_handler(&self.html_dir))
			.merge(state_utils_routes());
		// .layer(NormalizePathLayer::trim_trailing_slash());

		#[cfg(all(debug_assertions, feature = "reload"))]
		let reload_handle = {
			let (reload_layer, reload_handle) =
				Self::get_reload(&self.html_dir);
			router = router.layer(reload_layer);
			reload_handle
		};
		// let router = ServiceExt::<Request>::into_make_service(
		// 	NormalizePathLayer::trim_trailing_slash().layer(router),
		// );

		// get debug info for each request and response
		// router = router.layer(
		// 	TraceLayer::new_for_http()
		// 		.make_span_with(
		// 			trace::DefaultMakeSpan::new().level(Level::INFO),
		// 		)
		// 		.on_response(
		// 			trace::DefaultOnResponse::new().level(Level::INFO),
		// 		),
		// );


		#[cfg(feature = "lambda")]
		run_lambda(router).await?;
		#[cfg(not(feature = "lambda"))]
		{
			init_axum_tracing();
			let listener =
				tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
			tracing::info!("\nlistening on http://{}", listener.local_addr()?);
			axum::serve(listener, router.into_make_service()).await?;
		}

		#[cfg(all(debug_assertions, feature = "reload"))]
		reload_handle.await??;

		Ok(())
	}
	#[cfg(all(debug_assertions, feature = "reload"))]
	fn get_reload(
		html_dir: &std::path::Path,
	) -> (tower_livereload::LiveReloadLayer, JoinHandle<Result<()>>) {
		use beet_fs::prelude::FsWatcher;

		let livereload = tower_livereload::LiveReloadLayer::new();
		let reload = livereload.reloader();
		let html_dir = html_dir.to_path_buf();

		let reload_handle = tokio::spawn(async move {
			let mut rx = FsWatcher {
				cwd: html_dir,
				// no filter because any change in the html dir should trigger a reload
				..Default::default()
			}
			.watch()?;
			while let Some(ev) = rx.recv().await? {
				if ev.has_mutate() {
					println!("html files changed, reloading wasm...");
					reload.reload();
					// println!("{}", events);
					// this2.print_start();
				}
			}
			Ok(())
		});
		(livereload, reload_handle)
	}
}
