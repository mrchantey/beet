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
	pub fn new(src: impl Into<PathBuf>) -> Self { Self { src: Some(src.into()) } }
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
		let recompile = move || {
			send.send(TemplateReloaderMessage::Recompile)?;
			Ok(())
		};
		let builder = BuildRsxTemplateMap::new(&src);
		let dst = builder.dst.clone();

		let _handle = tokio::spawn(async move {
			TemplateWatcher::new(builder, reload, recompile)?
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
	// roots: Query<&RsxRoot>,
) {
	while let Ok(recv) = template_reload.recv.try_recv() {
		match recv {
			TemplateReloaderMessage::Reload => {
				let map = RsxTemplateMap::load(&template_reload.dst).unwrap();
				for (loc, root) in map.iter() {
					root.node.visit(|node| {
						let loc = 


						todo!("assign to existing nodes");
						// println!("{:?}", node);
					});
				}
			}
			TemplateReloaderMessage::Recompile => {
				println!("recompilation required, exiting..");
				app_exit.send(AppExit::Success);
			}
		}
	}
}
