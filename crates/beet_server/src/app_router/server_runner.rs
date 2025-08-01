#[cfg(not(feature = "lambda"))]
use beet_core::prelude::*;
use beet_rsx::as_beet::ResultExtDisplay;
#[allow(unused_imports)]
use beet_rsx::prelude::*;
use bevy::prelude::*;
use tracing::Level;
// use beet_router::types::RouteFunc;
#[allow(unused_imports)]
use crate::prelude::*;
use clap::Parser;
use clap::Subcommand;


/// Cli args for running a beet server.
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct ServerRunner {
	/// Specify the router mode
	#[command(subcommand)]
	pub mode: Option<RouterMode>,
	/// The tracing level to use for the app.
	#[arg(long, default_value = "info")]
	// tracing: Level::WARN,
	pub tracing: Level,
}
impl Default for ServerRunner {
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

impl ServerRunner {
	// #[cfg(target_arch = "wasm32")]
	// pub fn from_url_params() -> anyhow::Result<Self> {
	// 	// TODO actually parse from search params
	// 	Ok(Self {
	// 		is_static: false,
	// 		html_dir: "".into(),
	// 	})
	// }

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
	#[cfg(target_arch = "wasm32")]
	fn run(self, _: App) -> Result {
		todo!("wasm runner");
	}
	#[cfg(not(target_arch = "wasm32"))]
	fn run(self, mut app: App) -> Result {
		#[cfg(not(feature = "lambda"))]
		init_pretty_tracing(bevy::log::Level::DEBUG);

		let mode = self.mode.unwrap_or_default();
		if let RouterMode::Ssg = mode {}

		match mode {
			RouterMode::ExportHtml => {
				return self.export_html(&mut app);
			}
			RouterMode::Ssg => {
				// despawn all static endpoints, they will be loaded from the html dir
				self.export_html(&mut app)?;
				for (entity, _) in app
					.world_mut()
					.run_system_cached(ResolvedEndpoint::collect_static_get)?
					.into_iter()
				{
					app.world_mut().entity_mut(entity).despawn();
				}
			}
			RouterMode::Ssr => {}
		}
		#[cfg(feature = "axum")]
		{
			AxumRunner::new(self).run(app)
		}
		#[cfg(not(feature = "axum"))]
		todo!("hyper router");
	}

	/// Export static html files
	#[cfg(not(target_arch = "wasm32"))]
	#[tokio::main]
	async fn export_html(&self, app: &mut App) -> Result {
		let html = collect_html(app.world_mut()).await?;

		for (path, html) in html {
			debug!("Exporting html to {}", path);
			FsExt::write(path, &html)?;
		}
		Ok(())
	}
}
