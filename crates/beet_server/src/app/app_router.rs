use crate::prelude::*;
use axum::Router;
use beet_router::prelude::ClientIsland;
use beet_router::prelude::ClientIslandMap;
use beet_template::as_beet::bevybail;
use bevy::prelude::*;
// use beet_router::types::RouteFunc;
use clap::Parser;
use clap::Subcommand;
use http_body_util::BodyExt;
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
	/// Directory for temp static files like client islands.
	#[arg(long, default_value = "target")]
	pub static_dir: WsPathBuf,
}
impl Default for AppRouterConfig {
	fn default() -> Self {
		Self {
			mode: None,
			html_dir: "target/client".into(),
			static_dir: "target".into(),
		}
	}
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

// so dirty we need cleaner solution, ReactiveApp in general sucks
fn set_app() {
	#[cfg(feature = "build")]
	ReactiveApp::set_create_app(|| {
		let mut app = App::new();
		app.add_plugins((
			beet_build::prelude::StaticScenePlugin,
			TemplatePlugin,
		));
		app
	});
}

impl AppRouter<()> {
	/// The default app router parses cli arguments which is not desired in tests.
	pub fn test() -> Self {
		set_app();
		Self {
			config: default(),
			router: default(),
			static_routes: default(),
			state: default(),
			tracing: Level::WARN,
		}
	}
}

impl<S: 'static + Clone + Send + Sync> AppRouter<S> {
	pub fn new(state: S) -> Self {
		set_app();
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
	/// Add any axum route or bundle route to the router.
	pub fn add_route<M>(
		mut self,
		info: impl Into<RouteInfo>,
		route: impl IntoBeetRoute<M, State = S>,
	) -> Self
	where
		S: 'static + Send + Sync + Clone,
	{
		self.router = route.add_beet_route(self.router, info.into());
		self
	}
	pub fn add_plugins<M>(
		mut self,
		plugin: impl IntoRoutePlugins<S, M>,
	) -> Self {
		plugin.add_to_router(&mut self);
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
			Some(RouterMode::ExportStatic) => self.export_static(&config).await,
			_ => self.serve().await,
		}
	}

	/// Export static html files and client islands.
	pub async fn export_static(self, config: &AppRouterConfig) -> Result {
		let html_dir = config.html_dir.into_abs();
		let static_dir = config.static_dir.into_abs();

		self.static_routes
			.iter()
			.map(async |route| -> Result {
				let html_dir = html_dir.clone();
				let html = self.render_route(route).await?;
				let route_path =
					html_dir.join(&route.path.as_relative()).join("index.html");
				FsExt::write(&route_path, html)?;
				Ok(())
			})
			.xmap(futures::future::try_join_all)
			.await?;

		let islands = self
			.static_routes
			.iter()
			.map(async |route| -> Result<(RouteInfo, Vec<ClientIsland>)> {
				let islands = self.get_client_islands(route).await?;
				Ok((route.clone(), islands))
			})
			.xmap(futures::future::try_join_all)
			.await?;

		let islands = ClientIslandMap::new(islands);
		islands.write(&static_dir)?;
		let num_islands = islands.values().map(|v| v.len()).sum::<usize>();

		tracing::info!(
			"Exported {} html files and {} client islands to {}",
			self.static_routes.len(),
			num_islands,
			html_dir.display()
		);

		Ok(())
	}

	pub async fn render_route(&self, route: &RouteInfo) -> Result<String> {
		let router = self.router.clone().with_state(self.state.clone());
		let res = router
			.clone()
			.oneshot(
				axum::http::Request::builder()
					.uri(route.path.to_string_lossy().to_string())
					.body(axum::body::Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();
		if !res.status().is_success() {
			let status = res.status();
			let body = res
				.into_body()
				.collect()
				.await
				.unwrap_or_default()
				.to_bytes()
				.to_vec();
			let msg = String::from_utf8(body)?;

			bevybail!(
				"Failed to render route {}\n{status}: {msg}",
				route.path.to_string_lossy(),
			);
		}
		let body = res.into_body().collect().await?.to_bytes().to_vec();
		let html = String::from_utf8(body.to_vec())?;
		Ok(html)
	}

	pub async fn get_client_islands(
		&self,
		route: &RouteInfo,
	) -> Result<Vec<ClientIsland>> {
		let route_info = ClientIslandPlugin::route_info(route);
		let ron = self.render_route(&route_info).await?;
		let islands: Vec<ClientIsland> =
			beet_common::exports::ron::de::from_str(&ron).map_err(|e| {
				AppError::internal_error(format!(
					"Failed to deserialize client islands: {}",
					e
				))
			})?;
		Ok(islands)
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


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn works() {
		AppRouter::test()
			.add_route("/", || {
				rsx! {
					<h1>Hello World</h1>
					<p>This is a test page.</p>
				}
			})
			// .add_plugin(PagesPlugin)
			.render_route(&"/".into())
			.await
			.unwrap()
			.xpect()
			.to_be("<!DOCTYPE html><html><head></head><body><h1>Hello World</h1><p>This is a test page.</p></body></html>");
	}
}
