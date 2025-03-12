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
		let recompile = move || {
			send.send(TemplateReloaderMessage::Recompile)?;
			Ok(())
		};
		let builder = BuildTemplateMap::new(&src);
		let dst = builder.templates_map_path.clone();

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


/// Here we don't care at all about [`TreeIdx`] because any
/// matching [`GlobalRsxIdx`] will be updated.
#[allow(unused)]
fn handle_recv(
	template_reload: Res<TemplateReload>,
	mut commands: Commands,
	mut app_exit: EventWriter<AppExit>,
	mut text_query: Query<(&GlobalRsxIdx, &mut Text)>,
	mut other_query: Query<EntityMut, (Without<Text>, With<GlobalRsxIdx>)>,
) {
	while let Ok(recv) = template_reload.recv.try_recv() {
		match recv {
			TemplateReloaderMessage::Reload => {
				let map = RsxTemplateMap::load(&template_reload.dst).unwrap();
				for (loc, root) in map.iter() {
					let loc_hash = loc.into_hash();
					root.node.visit(|template_node| {
						let loc = GlobalRsxIdx::new(
							loc_hash,
							template_node.rsx_idx(),
						);
						match template_node {
							RsxTemplateNode::Text { idx, value } => {
								for (_, mut text) in text_query
									.iter_mut()
									.filter(|entity| *entity.0 == loc)
								{
									text.0 = value.clone();
								}
							}
							_ => {
								for entity in
									other_query.iter_mut().filter(|entity| {
										entity.get::<GlobalRsxIdx>()
											== Some(&loc)
									}) {
									// println!(
									// 	"gonna change this entity: {:?}\n{:#?}",
									// 	entity, template_node
									// );
								}
							}
						}
					});
				}
			}
			TemplateReloaderMessage::Recompile => {
				println!("recompilation required, exiting..");
				// seems it doesnt actu
				app_exit.send(AppExit::Success);
				std::process::exit(0);
			}
		}
	}
}
