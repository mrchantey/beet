use crate::prelude::*;
use anyhow::Result;
use axum::Router;
use std::path::PathBuf;
// use tower::Layer;
// use tower_http::normalize_path::NormalizePath;
// use tower_http::normalize_path::NormalizePathLayer;


/// The main server struct for Beet.
/// By default a file server will be used as a fallback.
pub struct BeetServer {
	pub html_dir: PathBuf,
	pub router: Router,
}

impl Default for BeetServer {
	fn default() -> Self {
		Self {
			html_dir: "target".into(),
			router: Router::default(),
		}
	}
}

impl BeetServer {
	/// Server the provided router, adding
	/// a fallback file server with live reload.
	pub async fn serve(self) -> Result<()> {
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


		#[cfg(feature = "lambda")]
		run_lambda(router).await?;
		#[cfg(not(feature = "lambda"))]
		{
			init_axum_tracing();
			let listener =
				tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
			tracing::info!("\nlistening on http://{}", listener.local_addr()?);
			axum::serve(listener, router).await?;
		}

		#[cfg(all(debug_assertions, feature = "reload"))]
		reload_handle.join().unwrap()?;

		Ok(())
	}
	#[cfg(all(debug_assertions, feature = "reload"))]
	fn get_reload(
		html_dir: &std::path::Path,
	) -> (
		tower_livereload::LiveReloadLayer,
		std::thread::JoinHandle<Result<()>>,
	) {
		use sweet::prelude::FsWatcher;

		let livereload = tower_livereload::LiveReloadLayer::new();
		let reload = livereload.reloader();
		let html_dir = html_dir.to_path_buf();

		let reload_handle = std::thread::spawn(move || -> Result<()> {
			FsWatcher {
				cwd: html_dir,
				// no filter because any change in the html dir should trigger a reload
				..Default::default()
			}
			.watch_blocking(move |e| {
				if e.has_mutate() {
					println!("html files changed, reloading wasm...");
					reload.reload();
					// println!("{}", events);
					// this2.print_start();
				}
				Ok(())
			})
		});
		(livereload, reload_handle)
	}
}
