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
		app.insert_resource(TemplateReload {
			recv,
			dst: builder.dst.clone(),
		});

		let _handle = tokio::spawn(async move {
			TemplateWatcher::new(builder, reload, recompile)?
				.watch()
				.await
		});
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


impl TemplateReload {
	pub fn reload(&self) {
		let mut template_map = RsxTemplateMap::load(&self.dst).unwrap();
	}
}


fn handle_recv(
	template_reload: Res<TemplateReload>,
	mut commands: Commands,
	mut app_exit: EventWriter<AppExit>,
) {
	while let Ok(recv) = template_reload.recv.try_recv() {
		match recv {
			TemplateReloaderMessage::Reload => template_reload.reload(),
			TemplateReloaderMessage::Recompile => {
				println!("recompilation required, exiting..");
				app_exit.send(AppExit::Success);
			}
		}
	}
}
