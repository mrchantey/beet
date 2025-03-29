use std::path::PathBuf;

use crate::prelude::*;
use anyhow::Result;
use axum::Router;



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
			router: Router::new().merge(state_utils_routes()),
		}
	}
}

impl BeetServer {
	pub async fn serve_without_fs(self) -> Result<()> {
		#[cfg(feature = "lambda")]
		return run_lambda(self.router).await;
		#[cfg(not(feature = "lambda"))]
		return run_axum(self.router).await;
	}
	/// Server the provided router, adding
	/// a fallback file server with live reload.
	pub async fn serve(mut self) -> Result<()> {
		self.router = self
			.router
			.fallback_service(file_and_error_handler(&self.html_dir));

		#[cfg(all(debug_assertions, feature = "reload"))]
		let reload_handle = {
			let (reload_layer, reload_handle) = self.get_reload();
			self.router = self.router.layer(reload_layer);
			reload_handle
		};

		#[cfg(feature = "lambda")]
		run_lambda(self.router).await?;
		#[cfg(not(feature = "lambda"))]
		run_axum(self.router).await?;

		#[cfg(all(debug_assertions, feature = "reload"))]
		reload_handle.join().unwrap()?;

		Ok(())
	}
	#[cfg(all(debug_assertions, feature = "reload"))]
	fn get_reload(
		&self,
	) -> (
		tower_livereload::LiveReloadLayer,
		std::thread::JoinHandle<Result<()>>,
	) {
		use sweet::prelude::FsWatcher;

		let livereload = tower_livereload::LiveReloadLayer::new();
		let reload = livereload.reloader();
		let html_dir = self.html_dir.clone();

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
