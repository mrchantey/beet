use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use bevy::prelude::*;
use bevy::scene::ron;
use bevy::scene::serde::SceneSerializer;


pub fn apply_client_islands(world: &mut World) -> Result {
	let documents = world.run_system_cached(document_islands)?;
	let type_registry = world.resource::<AppTypeRegistry>();
	let type_registry = type_registry.read();
	let filter = world.resource::<ClientIslandRegistry>().filter();
	// println!("filter: {:?}", type_registry.get_type_data());
	let document_scenes = documents
		.into_iter()
		.map(|(document, islands)| {
			let scene = DynamicSceneBuilder::from_world(world)
				.with_component_filter(filter.clone())
				.extract_entities(islands.into_iter())
				.build();
			let scene_serializer = SceneSerializer::new(&scene, &type_registry);
			#[cfg(debug_assertions)]
			let scene = {
				let pretty_config = ron::ser::PrettyConfig::default()
					.indentor("  ".to_string())
					.new_line("\n".to_string());
				ron::ser::to_string_pretty(&scene_serializer, pretty_config)
					.expect("failed to serialize scene")
			};
			#[cfg(not(debug_assertions))]
			let scene = ron::ser::to_string(&scene_serializer)
				.expect("failed to serialize scene");
			(document, scene)
		})
		.collect::<Vec<_>>();
	drop(type_registry);

	let script_type = world
		.resource::<HtmlConstants>()
		.client_islands_script_type
		.clone();

	for (document, scene) in document_scenes.into_iter() {
		world.entity_mut(document).with_child((
			ElementNode::open(),
			NodeTag::new("script"),
			HtmlHoistDirective::Body,
			related!(
				Attributes[(
					AttributeKey::new("type"),
					TextNode::new(script_type.clone()),
				),]
			),
			children![TextNode::new(scene)],
		));
	}


	Ok(())
}


/// Returns each [`HtmlDocument`] entity and a list of its [`ClientIsland`] entities
fn document_islands(
	query: Query<Entity, Added<HtmlDocument>>,
	children: Query<&Children>,
	islands: Query<
		Entity,
		Or<(With<ClientLoadDirective>, With<ClientOnlyDirective>)>,
	>,
) -> Vec<(Entity, Vec<Entity>)> {
	query
		.iter()
		.filter_map(|document| {
			let islands = children
				.iter_descendants_inclusive(document)
				.filter(|e| islands.contains(*e))
				.collect::<Vec<_>>();
			if islands.is_empty() {
				None
			} else {
				Some((document, islands))
			}
		})
		.collect()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[template]
	#[derive(Reflect)]
	pub fn MyTemplate(foo: u32) -> impl Bundle {
		let _ = foo;
		()
	}

	fn parse(app: &mut App, bundle: impl Bundle) -> Result<String> {
		let entity = app.world_mut().spawn((HtmlDocument, bundle)).id();
		let reg = app.world().resource::<AppTypeRegistry>();
		reg.read()
			.get_type_info(
				std::any::TypeId::of::<ClientIslandRoot<MyTemplate>>(),
			)
			// OnSpawnDeferred not triggered yet
			.xpect_none();

		app.update();
		let reg = app.world().resource::<AppTypeRegistry>();
		reg.read()
			.get_type_info(
				std::any::TypeId::of::<ClientIslandRoot<MyTemplate>>(),
			)
			.xpect_some();

		app.world_mut()
			.run_system_cached_with(render_fragment, entity)?
			.xok()
	}

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(ApplyDirectivesPlugin::default());

		parse(&mut app, rsx! { <MyTemplate foo=3 client:only /> })
			.unwrap()
			.xpect_snapshot();
	}
}
