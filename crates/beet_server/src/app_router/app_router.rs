use crate::prelude::*;
use axum::Router;
use beet_router::prelude::ClientIsland;
use beet_router::prelude::ClientIslandMap;
use beet_rsx::as_beet::bevybail;
use bevy::app::Plugins;
use bevy::prelude::*;
use std::path::PathBuf;
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
pub struct AppRouterArgs {
	/// Specify the router mode
	#[command(subcommand)]
	mode: Option<RouterMode>,
	/// Location of the beet.toml config file
	#[arg(long)]
	beet_config: Option<PathBuf>,
}
impl Default for AppRouterArgs {
	fn default() -> Self {
		Self {
			mode: None,
			beet_config: None,
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

impl AppRouterArgs {
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
pub struct AppRouter<S = AppRouterState> {
	pub router: Router<S>,
	/// The Router state, added to the router when calling [`Self::serve`].
	pub state: S,
	pub tracing: Level,
	/// A list of routes that can be used to generate static html files.
	pub static_routes: Vec<RouteInfo>,
	/// cli arguments passed in.
	pub args: AppRouterArgs,
}

impl Default for AppRouter<AppRouterState> {
	fn default() -> Self { Self::new(Default::default()) }
}

impl AppRouter<AppRouterState> {
	/// The default app router parses cli arguments, trying to consume
	/// test args if present
	pub fn test() -> Self {
		Self {
			router: default(),
			static_routes: default(),
			state: default(),
			// we dont want tracing in tests
			tracing: Level::WARN,
			args: default(),
		}
		.set_bevy_plugins(|app: &mut App| {
			// usually we dont want to load snippets in tests,
			// but this can be overridden by calling set_bevy_plugins
			// later
			app.insert_resource(TemplateFlags::None);
		})
	}
}

impl<S: DerivedAppState> AppRouter<S> {
	fn app_state(&self) -> &AppRouterState { self.state.as_ref() }
	fn app_state_mut(&mut self) -> &mut AppRouterState { self.state.as_mut() }
	pub fn new(mut state: S) -> Self {
		let args = AppRouterArgs::parse();
		let app_state = state.as_mut();
		if app_state.template_config.is_none() {
			app_state.template_config = Some(
				BeetConfigFile::try_load_or_default::<TemplateConfig>(
					args.beet_config.as_deref(),
				)
				.unwrap_or_exit(),
			);
		}

		Self {
			args,
			router: default(),
			static_routes: default(),
			state,
			tracing: Level::INFO,
		}
	}
}

impl<'a, S: DerivedAppState> AppRouter<S> {
	/// Add any axum route or bundle route to the router.
	pub fn add_route<M>(
		mut self,
		info: impl Into<RouteInfo>,
		route: impl IntoBeetRoute<M, State = S>,
	) -> Self {
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
	pub fn set_bevy_plugins<M>(
		mut self,
		plugins: impl 'static + Clone + Send + Sync + Plugins<M>,
	) -> Self {
		self.app_state_mut().set_plugins(plugins);
		self
	}


	#[tokio::main]
	pub async fn run(self) -> Result<()> {
		// lambda does its own tracing thing
		#[cfg(not(feature = "lambda"))]
		init_pretty_tracing(bevy::log::Level::DEBUG);
		match self.args.mode {
			Some(RouterMode::ExportStatic) => self.export_static().await,
			_ => self.serve().await,
		}
	}

	/// Export static html files and client islands.
	pub async fn export_static(self) -> Result {
		let template_config =
			self.app_state().template_config.clone().unwrap_or_default();
		let html_dir = template_config.workspace.html_dir.into_abs();
		let static_dir =
			template_config.workspace.client_islands_path.into_abs();

		let html = self
			.static_routes
			.iter()
			.map(async |route| -> Result<(AbsPathBuf, String)> {
				let html_dir = html_dir.clone();
				let html = self.render_route(route).await?;
				let route_path =
					html_dir.join(&route.path.as_relative()).join("index.html");

				Ok((route_path, html))
			})
			.xmap(futures::future::try_join_all)
			.await?;
		// write files all at once to avoid triggering file watcher multiple times
		for (path, html) in html {
			FsExt::write(path, html)?;
		}


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

	pub(super) async fn get_client_islands(
		&self,
		route: &RouteInfo,
	) -> Result<Vec<ClientIsland>> {
		// convert /foobar into /__client_islands/foobar
		let route_info = ClientIslandRouterPlugin::route_info(route);
		let ron = self.render_route(&route_info).await?;

		let islands: Vec<ClientIsland> =
			beet_core::exports::ron::de::from_str(&ron).map_err(|e| {
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
		let template_config =
			self.app_state().template_config.clone().unwrap_or_default();
		let html_dir = template_config.workspace.html_dir.into_abs();

		#[allow(unused_mut)]
		let mut router = self
			.router
			.with_state(self.state)
			.merge(state_utils_routes());
		// .layer(NormalizePathLayer::trim_trailing_slash());

		match self.args.mode {
			Some(RouterMode::Ssg) | None => {
				router =
					router.fallback_service(file_and_error_handler(&html_dir));
			}
			_ => {}
		};

		#[cfg(all(debug_assertions, feature = "reload"))]
		let reload_handle = match self.args.mode {
			Some(RouterMode::Ssg) | None => {
				let (reload_layer, reload_handle) = Self::get_reload(html_dir);
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
