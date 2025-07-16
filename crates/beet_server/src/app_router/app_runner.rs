#[cfg(not(feature = "lambda"))]
use beet_core::prelude::*;
use beet_rsx::as_beet::ResultExtDisplay;
use beet_rsx::prelude::*;
use bevy::prelude::*;
use tracing::Level;
// use beet_router::types::RouteFunc;
use crate::prelude::*;
use clap::Parser;
use clap::Subcommand;


// use tower::Layer;
// use tower_http::normalize_path::NormalizePath;
// use tower_http::normalize_path::NormalizePathLayer;

/// Cli args parser when running an [`AppRouter`].
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct AppRunner {
	/// Specify the router mode
	#[command(subcommand)]
	pub mode: Option<RouterMode>,
	/// The tracing level to use for the app.
	#[arg(long, default_value = "info")]
	// tracing: Level::WARN,
	pub tracing: Level,
}
impl Default for AppRunner {
	fn default() -> Self {
		Self {
			mode: None,
			#[cfg(test)]
			tracing: Level::WARN,
			#[cfg(not(test))]
			tracing: Level::INFO,
		}
	}
}


#[derive(Default, Copy, Clone, Subcommand)]
pub enum RouterMode {
	/// Do not add static routes to the router, instead loading them from
	/// the `html_dir`.
	#[default]
	Ssg,
	/// Add static routes to the router, rendering them on each request.
	Ssr,
	/// Export static html and wasm scene, then exit.
	ExportHtml,
}

impl AppRunner {
	#[cfg(target_arch = "wasm32")]
	pub fn from_url_params() -> anyhow::Result<Self> {
		// TODO actually parse from search params
		Ok(Self {
			is_static: false,
			html_dir: "".into(),
		})
	}

	pub fn runner(mut app: App) -> AppExit {
		app.init();
		app.update();
		// allow setup to decide to exit
		match app.should_exit() {
			Some(exit) => return exit,
			None => {
				Self::parse().run(app).unwrap_or_exit();
				AppExit::Success
			}
		}
	}

	fn run(self, app: App) -> Result {
		#[cfg(not(feature = "lambda"))]
		init_pretty_tracing(bevy::log::Level::DEBUG);

		match self.mode.clone().unwrap_or_default() {
			RouterMode::ExportHtml => self.export_html(app),
			_ => {
				#[cfg(feature = "axum")]
				{
					AxumRunner::new(self).run(app)
				}
				#[cfg(not(feature = "axum"))]
				todo!("a simple beet router");
			}
		}
	}

	/// Export static html files and client islands.
	#[tokio::main]
	async fn export_html(self, mut app: App) -> Result {
		let workspace_config = app.world().resource::<WorkspaceConfig>();
		let html_dir = workspace_config.html_dir.into_abs();
		let html = app
			.world_mut()
			.query_filtered_once::<(Entity, &RouteInfo), With<StaticRoute>>()
			.into_iter()
			.map(|(entity, info)| (entity, info.clone()))
			.collect::<Vec<_>>()
			.into_iter()
			.map(|(entity, _)| {
				let instance = app.world_mut().run_system_cached_with(
					RouteInstance::from_entity,
					entity,
				)?;
				Ok(instance)
			})
			.collect::<Result<Vec<_>>>()?
			.into_iter()
			.map(async |instance| -> Result<(AbsPathBuf, String)> {
				let info = instance.route_info.clone();
				let route_path =
					html_dir.join(&info.path.as_relative()).join("index.html");
				let response = instance.call(info.clone().into()).await?;
				let html = response.body_str().ok_or_else(|| {
					AppError::internal_error(format!(
						"Failed to get body from response for route {}",
						info.path
					))
				})?;

				Ok((route_path, html))
			})
			.xmap(futures::future::try_join_all)
			.await?;

		// write files all at once to avoid triggering file watcher multiple times
		for (path, html) in html {
			FsExt::write(path, &html)?;
		}
		Ok(())
	}
}
