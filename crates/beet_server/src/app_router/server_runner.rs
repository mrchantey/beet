#[cfg(not(feature = "lambda"))]
use beet_core::prelude::*;
use beet_rsx::as_beet::ResultExtDisplay;
#[allow(unused_imports)]
use beet_rsx::prelude::*;
use bevy::prelude::*;
// use beet_router::types::RouteFunc;
#[allow(unused_imports)]
use crate::prelude::*;
use clap::Parser;
use clap::Subcommand;


/// Cli args for running a beet server.
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct ServerRunner {
	/// Only export the static html files to the [`WorkspaceConfig::html_dir`],
	/// and immediately exit.
	#[arg(long)]
	pub export_static: bool,
	/// Specify the router mode
	#[command(subcommand)]
	pub mode: Option<RenderMode>,
}
impl Default for ServerRunner {
	fn default() -> Self {
		Self {
			export_static: false,
			mode: None,
		}
	}
}


#[derive(Default, Copy, Clone, Resource, Subcommand)]
pub enum RenderMode {
	/// Static html routes will be skipped, using the [`bucket_handler`] fallback
	/// to serve files from the bucket.
	#[default]
	Ssg,
	/// All static html [`RouteHandler`] funcs will run instead of using the [`bucket_handler`].
	Ssr,
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
		PrettyTracing::default().init();

		let mode = self.mode.unwrap_or_default();
		app.insert_resource(mode.clone());

		if self.export_static {
			return self.export_static(&mut app);
		}

		#[cfg(feature = "axum")]
		{
			AxumRunner::new(self).run(app)
		}
		#[cfg(not(feature = "axum"))]
		todo!("hyper router");
	}

	/// Export static html files, with the router in SSG mode.
	#[cfg(not(target_arch = "wasm32"))]
	#[tokio::main]
	async fn export_static(&self, app: &mut App) -> Result {
		// force ssr to ensure static handlers run
		app.insert_resource(RenderMode::Ssr);
		let html = collect_html(app.world_mut()).await?;

		for (path, html) in html {
			trace!("Exporting html to {}", path);
			FsExt::write(path, &html)?;
		}
		Ok(())
	}
}
