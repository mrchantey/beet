#[allow(unused_imports)]
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
// use beet_router::types::RouteFunc;
#[allow(unused_imports)]
use crate::prelude::*;


/// Cli args for running a beet server.
#[cfg_attr(feature = "serde", derive(clap::Parser))]
#[cfg_attr(feature = "serde", command(version, about, long_about = None))]
pub struct ServerRunner {
	/// Only export the static html files to the [`WorkspaceConfig::html_dir`],
	/// and immediately exit.
	#[cfg_attr(feature = "serde", arg(long))]
	pub export_static: bool,
	#[cfg_attr(feature = "serde", clap(flatten))]
	#[allow(unused)]
	pub(crate) config_overrides: ConfigOverrides,
	/// Specify the router mode
	#[cfg_attr(feature = "serde", command(subcommand))]
	pub mode: Option<RenderMode>,
}
impl Default for ServerRunner {
	fn default() -> Self {
		Self {
			export_static: false,
			config_overrides: Default::default(),
			mode: None,
		}
	}
}

#[derive(Debug, Default, Copy, Clone, Resource, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(clap::Subcommand))]
pub enum RenderMode {
	/// Static html routes will be skipped, using the [`bucket_handler`] fallback
	/// to serve files from the bucket.
	#[default]
	Ssg,
	/// All static html [`RouteHandler`] funcs will run instead of using the [`bucket_handler`].
	Ssr,
}

impl ServerRunner {
	pub fn runner(app: App) -> AppExit {
		#[cfg(not(feature = "serde"))]
		todo!("wasm runner");
		#[cfg(feature = "serde")]
		{
			use clap::Parser;
			Self::parse().run(app).unwrap_or_exit();
		}
		AppExit::Success
	}
	#[cfg(target_arch = "wasm32")]
	fn run(self, _: App) -> Result {
		todo!("wasm runner");
	}
	#[cfg(not(target_arch = "wasm32"))]
	#[tokio::main]
	async fn run(self, mut app: App) -> Result {
		PrettyTracing::default().init();
		app.add_plugins(self.config_overrides);

		if self.export_static {
			app.insert_resource(RenderMode::Ssr);
		} else if !app.world().contains_resource::<RenderMode>() {
			app.insert_resource(self.mode.unwrap_or_default());
		}
		app.init();

		let world = std::mem::take(app.world_mut());
		*app.world_mut() = AsyncActionSet::collect_and_run(world).await;

		app.update();
		if let Some(exit) = app.should_exit() {
			exit.into_result()
		} else if self.export_static {
			Self::export_static(&mut app).await
		} else {
			#[cfg(feature = "axum")]
			{
				AxumRunner::new().run(app.world_mut()).await
			}
			#[cfg(not(feature = "axum"))]
			todo!("hyper router");
		}
	}

	/// Export static html files, with the router in SSG mode.
	#[cfg(not(target_arch = "wasm32"))]
	async fn export_static(app: &mut App) -> Result {
		let html = collect_html(app.world_mut()).await?;

		for (path, html) in html {
			trace!("Exporting html to {}", path);
			FsExt::write(path, &html)?;
		}
		Ok(())
	}
}
