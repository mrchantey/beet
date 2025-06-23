use beet_bevy::prelude::*;
use beet_common::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientIsland {
	pub template: TemplateSerde,
	/// If the template is [`ClientOnlyDirective`], this will be true.
	pub mount: bool,
	pub tracker: RustyTracker, // pub route: RouteInfo,
}


pub fn collect_client_islands(
	In(root): In<Entity>,
	children: Query<&Children>,
	islands: Query<(
		&TemplateSerde,
		&ItemOf<TemplateNode, RustyTracker>,
		Option<&ClientOnlyDirective>,
	)>,
) -> Vec<ClientIsland> {
	children
		.iter_descendants_inclusive(root)
		.filter_map(|entity| {
			islands
				.get(entity)
				.ok()
				.map(|(template, tracker, client_only)| {
					let mount = client_only.is_some();
					ClientIsland {
						template: template.clone(),
						tracker: tracker.value.clone(),
						mount,
						// route: RouteInfo::from_tracker(tracker),
					}
				})
		})
		.collect()
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_template::prelude::TemplatePlugin;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use serde::Deserialize;
	use serde::Serialize;
	use sweet::prelude::*;

	use beet_template::as_beet::*;

	#[template]
	#[derive(Serialize, Deserialize)]
	pub fn MyTemplate(foo: u32) -> impl Bundle {
		let _ = foo;
		()
	}

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(TemplatePlugin);
		let entity = app
			.world_mut()
			.spawn(rsx! {
				<MyTemplate foo=3 client:only />
			})
			.id();
		app.world_mut()
			.run_system_once_with(collect_client_islands, entity)
			.unwrap()
			.xpect()
			.to_be(vec![ClientIsland {
				template: TemplateSerde::new(&MyTemplate { foo: 3 }),
				tracker: RustyTracker::new(0, 7960668749389905152),
				mount: true,
			}]);
	}
}
