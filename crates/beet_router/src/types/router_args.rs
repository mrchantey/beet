#[allow(unused_imports)]
use crate::prelude::*;
use beet_core::prelude::*;
use clap::Parser;
// use beet_router::types::RouteFunc;
#[allow(unused_imports)]
use crate::prelude::*;


/// Cli args for running a beet server.
#[cfg_attr(feature = "serde", derive(clap::Parser))]
#[cfg_attr(feature = "serde", command(version, about, long_about = None))]
pub struct RouterArgs {
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

impl RouterArgs {
	pub fn parse() -> Self { Parser::parse() }
}

impl Default for RouterArgs {
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

impl Plugin for RouterArgs {
	fn build(&self, app: &mut App) {
		PrettyTracing::default().init();
		app.add_plugins(self.config_overrides.clone());

		if self.export_static {
			app.insert_resource(RenderMode::Ssr)
				.add_systems(PostStartup, export_static);
		} else if !app.world().contains_resource::<RenderMode>() {
			app.insert_resource(self.mode.unwrap_or_default());
		}
	}
}

fn export_static(mut commands: AsyncCommands) {
	commands.run_local(async |world| -> Result {
		// wait for the server to be ready
		world.await_event::<Insert, RouteServer>().await;

		let html = collect_html(world.clone()).await?;

		for (path, html) in html {
			trace!("Exporting html to {}", path);
			fs_ext::write(path, &html)?;
		}
		world.write_message(AppExit::Success);
		Ok(())
	});
}
