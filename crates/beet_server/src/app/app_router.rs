use crate::prelude::*;
use axum::Router;
use bevy::prelude::Result;
// use beet_router::types::RouteFunc;
use clap::Parser;
use clap::Subcommand;
use http_body_util::BodyExt;
#[cfg(feature = "lambda")]
use lambda_http::Body;
#[cfg(all(debug_assertions, feature = "reload"))]
use tokio::task::JoinHandle;
use tower::util::ServiceExt;
use tracing::Level;

// use tower::Layer;
// use tower_http::normalize_path::NormalizePath;
// use tower_http::normalize_path::NormalizePathLayer;

/// Cli args parser when running an [`AppRouter`].
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct AppRouterConfig {
	/// Specify the router mode
	#[command(subcommand)]
	mode: Option<RouterMode>,
	/// The directory where the static html files will be exported and
	/// served from.
	#[arg(long, default_value = "target/client")]
	pub html_dir: WsPathBuf,
}

#[derive(Default, Subcommand)]
enum RouterMode {
	/// Do not add static routes to the router, instead loading them from
	/// the `html_dir`.
	#[default]
	Ssg,
	/// Add static routes to the router, rendering them on each request.
	Ssr,
	/// Export static html and wasm scene, then exit.
	ExportStatic,
}

impl AppRouterConfig {
	#[cfg(target_arch = "wasm32")]
	pub fn from_url_params() -> anyhow::Result<Self> {
		// TODO actually parse from search params
		Ok(Self {
			is_static: false,
			html_dir: "".into(),
		})
	}
}

/// An Axum Server with file based routing and a live reload dev server.
pub struct AppRouter<S = ()> {
	pub router: Router<S>,
	/// The Router state, added to the router when calling [`Self::serve`].
	pub state: S,
	pub tracing: Level,
	/// A list of routes that can be used to generate static html files.
	pub static_routes: Vec<RouteInfo>,
	pub config: AppRouterConfig,
}

impl Default for AppRouter<()> {
	fn default() -> Self { Self::new(Default::default()) }
}
impl<S: 'static + Clone + Send + Sync> AppRouter<S> {
	pub fn new(state: S) -> Self {
		Self {
			router: Router::new(),
			static_routes: Vec::new(),
			state,
			#[cfg(debug_assertions)]
			tracing: Level::INFO,
			#[cfg(not(debug_assertions))]
			tracing: Level::WARN,
			config: AppRouterConfig::parse(),
		}
	}
}

impl<S> AppRouter<S> {
	pub fn add_route<M>(
		mut self,
		info: impl Into<RouteInfo>,
		route: impl BundleRoute<M, State = S>,
	) -> Self
	where
		S: 'static + Send + Sync + Clone,
	{
		let info = info.into();
		self.router = self.router.route(
			&info.path.to_string(),
			route.into_method_router(info.method),
		);
		self
	}
	pub fn add_plugin(mut self, plugin: impl RouterPlugin<State = S>) -> Self {
		if plugin.is_static() {
			self.static_routes.extend(plugin.routes());
		}
		self.router = plugin.add_routes(self.router);
		self
	}
}

impl<'a, S> AppRouter<S>
where
	S: 'static + Send + Sync + Clone,
{
	pub fn run(self) -> Result {
		self.run_with_config(AppRouterConfig::parse())
	}
	#[tokio::main]
	pub async fn run_with_config(self, config: AppRouterConfig) -> Result<()> {
		match self.config.mode {
			Some(RouterMode::ExportStatic) => {
				self.export_static(&config.html_dir).await
			}
			_ => self.serve().await,
		}
	}

	pub async fn export_static(self, html_dir: &WsPathBuf) -> Result {
		let html_dir = html_dir.into_abs();
		let router = self.router.with_state(self.state);

		for route in &self.static_routes {
			let res = router
				.clone()
				.oneshot(
					axum::http::Request::builder()
						.uri("/test?name=world")
						.body(axum::body::Body::empty())
						.unwrap(),
				)
				.await
				.unwrap();
			let body = res.into_body().collect().await.unwrap().to_bytes();
			let html = String::from_utf8(body.to_vec()).unwrap();
			let route_path = html_dir.join(&route.path).with_extension("html");
			FsExt::write(&route_path, html)?;
		}
		tracing::info!(
			"Exported {} static html files to {}",
			self.static_routes.len(),
			html_dir.display()
		);

		Ok(())
	}

	/// Server the provided router, adding
	/// a fallback file server with live reload.
	pub async fn serve(self) -> Result<()> {
		#[allow(unused_mut)]
		let mut router = self
			.router
			.with_state(self.state)
			.merge(state_utils_routes());
		// .layer(NormalizePathLayer::trim_trailing_slash());

		match self.config.mode {
			Some(RouterMode::Ssg) | None => {
				router = router.fallback_service(file_and_error_handler(
					&self.config.html_dir,
				));
			}
			_ => {}
		};

		#[cfg(all(debug_assertions, feature = "reload"))]
		let reload_handle = match self.config.mode {
			Some(RouterMode::Ssg) | None => {
				let (reload_layer, reload_handle) =
					Self::get_reload(&self.config.html_dir);
				router = router.layer(reload_layer);
				Some(reload_handle)
			}
			_ => None,
		};

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
		if let Some(reload_handle) = reload_handle {
			reload_handle.await??;
		}

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
