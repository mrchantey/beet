use crate::prelude::*;
use beet_rsx::prelude::*;
use bevy::prelude::*;
use flume::Receiver;
use std::path::PathBuf;

#[derive(Default)]
pub struct BevyTemplateReloader {
	/// The source directory to watch, defaults to cwd.
	src: Option<PathBuf>,
}

impl BevyTemplateReloader {
	pub fn new(src: impl Into<PathBuf>) -> Self {
		Self {
			src: Some(src.into()),
		}
	}
}

impl Plugin for BevyTemplateReloader {
	fn build(&self, app: &mut App) {
		let src = self
			.src
			.clone()
			.unwrap_or_else(|| std::env::current_dir().unwrap());

		let (send, recv) = flume::unbounded::<TemplateReloaderMessage>();


		let send2 = send.clone();
		let reload = move || {
			send2.send(TemplateReloaderMessage::Reload)?;
			Ok(())
		};
		let builder = BuildTemplateMaps::new(&src);
		let templates_root_dir = builder.templates_root_dir.clone();
		let dst = builder.node_templates_path.clone();

		let recompile = move || {
			builder.build_and_write()?;
			send.send(TemplateReloaderMessage::Recompile)?;
			Ok(())
		};

		let _handle = tokio::spawn(async move {
			TemplateWatcher::new(templates_root_dir, reload, recompile)?
				.watch()
				.await
		});

		app.insert_resource(TemplateReload { recv, dst })
			.add_systems(Update, handle_recv);
	}
}

#[derive(Debug, Copy, Clone)]
enum TemplateReloaderMessage {
	Reload,
	Recompile,
}

#[derive(Resource)]
struct TemplateReload {
	pub recv: Receiver<TemplateReloaderMessage>,
	/// Location of the rsx-templates.ron file
	pub dst: PathBuf,
}


#[allow(unused)]
fn handle_recv(
	template_reload: Res<TemplateReload>,
	mut commands: Commands,
	mut app_exit: EventWriter<AppExit>,
	to_remove: Query<Entity, With<TreeIdx>>,
) {
	let rsx_idx = todo!(
		"this needs to be rearchitected, templates cannot keep track of the rsx idx"
	);
	while let Ok(recv) = template_reload.recv.try_recv() {
		match recv {
			TemplateReloaderMessage::Reload => {
				let map = NodeTemplateMap::load(&template_reload.dst).unwrap();
				todo!(
					"reload template just like html, we'll need to track the WebNode functions as entities"
				);
			}
			TemplateReloaderMessage::Recompile => {
				println!("recompilation required, exiting..");
				app_exit.write(AppExit::Success);
				std::process::exit(0);
			}
		}
	}
}
