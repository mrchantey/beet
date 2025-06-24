use beet_bevy::prelude::*;
use beet_common::prelude::*;
use beet_template::prelude::TemplatePlugin;
use beet_template::prelude::TreeIdx;
use bevy::ecs::system::RunSystemError;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientIsland {
	pub template: TemplateSerde,
	/// If the template is [`ClientOnlyDirective`], this will be true.
	pub mount: bool,
	pub tree_idx: TreeIdx, // pub route: RouteInfo,
}


impl ClientIsland {
	pub fn collect(bundle: impl Bundle) -> Result<Vec<Self>, RunSystemError> {
		let mut app = App::new();
		app.add_plugins(TemplatePlugin);
		let entity = app.world_mut().spawn(bundle).id();
		app.update();
		app.world_mut()
			.run_system_once_with(collect_client_islands, entity)
	}
}

pub fn collect_client_islands(
	In(root): In<Entity>,
	children: Query<&Children>,
	islands: Query<(&TemplateSerde, &TreeIdx, Option<&ClientOnlyDirective>)>,
) -> Vec<ClientIsland> {
	children
		.iter_descendants_inclusive(root)
		.filter_map(|entity| {
			islands
				.get(entity)
				.ok()
				.map(|(template, tree_idx, client_only)| {
					let mount = client_only.is_some();
					ClientIsland {
						template: template.clone(),
						tree_idx: *tree_idx,
						mount,
						// route: RouteInfo::from_tracker(tracker),
					}
				})
		})
		.collect()
}



#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod test {
	use crate::prelude::*;
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
		ClientIsland::collect(rsx! {
			<MyTemplate foo=3 client:only />
		})
		.unwrap()
		.xpect()
		.to_be(vec![ClientIsland {
			template: TemplateSerde::new(&MyTemplate { foo: 3 }),
			tree_idx: TreeIdx::new(0),
			mount: true,
		}]);
	}
}
