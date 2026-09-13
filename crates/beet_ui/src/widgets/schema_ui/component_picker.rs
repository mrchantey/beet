//! The component picker: the key half of a keyed map's add row, a `<select>`
//! over the components this binary registered.
//!
//! A [`MapSchema::Keyed`] map's keys are schema names, and for a scene's
//! component map those are the type paths the [`AppTypeRegistry`] holds, so
//! the picker lists every reflected component that is content: one with a
//! `ReflectComponent`, not [`Derived`](ReflectDerived), and not a relationship
//! target (a `Children`, which the world rebuilds from its sources and would
//! only corrupt if written). The options ride a [`ValueRebuild`] over the map,
//! so a component the entity already holds drops out of the list the moment it
//! is added.
use super::collection_edit::NewEntryKey;
use super::value_rebuild::RebuildKey;
use super::value_rebuild::ValueRebuild;
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::component::Components;
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::relationship::RelationshipAccessor;

/// The `<select>` a keyed map's add row chooses its next entry with: the
/// registry's components, labelled by short name (the full path where two
/// share one), minus the ones the map at `field` already holds.
///
/// Carries [`NewEntryKey`], so the add button beside it reads the chosen type
/// path as the key to insert under.
#[template(system)]
pub(in crate::widgets) fn ComponentPicker(
	/// The keyed map the picker adds to.
	#[prop]
	field: FieldRef,
	types: Res<AppTypeRegistry>,
	components: &Components,
) -> impl Bundle {
	let candidates = candidates(&types.read(), components);
	let rebuild = ValueRebuild::new(
		|map| vec![RebuildKey::Name(present(map).join("\n").into())],
		move |_resolver, map, _key| {
			let present = present(map);
			let options = candidates
				.iter()
				.filter(|(_, type_path)| !present.contains(type_path))
				.map(|(label, type_path)| {
					let (label, value) =
						(label.to_string(), type_path.to_string());
					rsx! { <option value={value}>{label}</option> }
						.any_snippet()
				})
				.collect::<Vec<_>>();
			Snippet::from_bundle(options.into_snippet())
		},
	);
	rsx! {
		<Select {NewEntryKey}>
			// a group rather than a bare holder: valid inside a `<select>` on the
			// web, and the terminal never lays out either
			<optgroup label="Components" {(field, rebuild)}/>
		</Select>
	}
}

/// The keys the map currently holds, sorted.
fn present(map: &Value) -> Vec<SmolStr> {
	map.as_map()
		.map(|map| map.0.keys().cloned().collect::<Vec<_>>())
		.unwrap_or_default()
		.xtap(|keys| keys.sort())
}

/// Every component a picker may add, as `(label, type path)` sorted by label:
/// the reflected components that are content, labelled by short path unless
/// another candidate shares it.
fn candidates(
	types: &bevy::reflect::TypeRegistry,
	components: &Components,
) -> Vec<(SmolStr, SmolStr)> {
	let is_target = |type_id| {
		components
			.get_valid_id(type_id)
			.and_then(|id| components.get_info(id))
			.is_some_and(|info| {
				matches!(
					info.relationship_accessor(),
					Some(RelationshipAccessor::RelationshipTarget { .. })
				)
			})
	};
	let paths = types
		.iter()
		.filter(|registration| {
			registration.data::<ReflectComponent>().is_some()
				&& registration.data::<ReflectDerived>().is_none()
				&& !is_target(registration.type_id())
		})
		.map(|registration| {
			let table = registration.type_info().type_path_table();
			(
				SmolStr::from(table.short_path()),
				SmolStr::from(table.path()),
			)
		})
		.collect::<Vec<_>>();
	let mut counts = HashMap::<SmolStr, usize>::default();
	for (short, _) in &paths {
		*counts.entry(short.clone()).or_default() += 1;
	}
	paths
		.into_iter()
		.map(|(short, path)| match counts[&short] {
			1 => (short, path),
			_ => (path.clone(), path),
		})
		.collect::<Vec<_>>()
		.xtap(|candidates| candidates.sort())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::widgets::schema_ui::test_ext;
	use beet_core::prelude::*;

	#[derive(Default, Component, Reflect)]
	#[reflect(Component, Default)]
	struct Health(u32);

	const NAME: &str = "bevy_ecs::name::Name";

	/// A form over a scene entity holding a `Name`, as the inspector binds
	/// one, in a world registering one more component.
	fn build() -> (World, Entity) {
		let mut world = test_ext::form_world();
		world
			.resource::<AppTypeRegistry>()
			.write()
			.register::<Health>();
		let root = world
			.spawn_template(rsx! {
				<div>
					<DynamicForm
						schema={ValueSchema::reference(ValueSchema::SCENE_ENTITY)}
						field={FieldRef::new(SceneEntities::entity_path(0))}
					/>
				</div>
			})
			.unwrap()
			.id();
		world
			.entity_mut(root)
			.insert(Document::new(SceneEntities::scene([(
				0,
				Map::new([(NAME, "a")]),
			)])));
		test_ext::settle_world(&mut world);
		(world, root)
	}

	/// The picker offers the registered components the map does not hold,
	/// by short name, and never a relationship target.
	#[beet_core::test]
	fn offers_the_missing_components() {
		let (mut world, root) = build();
		let html = test_ext::render_world(&mut world, root);
		html.clone()
			.xpect_contains("<option value=\"beet_ui::widgets::schema_ui::component_picker::test::Health\">Health</option>")
			.xpect_contains(">ChildOf</option>");
		html.clone().xnot().xpect_contains("Children</option>");
		html.xnot().xpect_contains(">Name</option>");
	}

	/// Choosing a component and pressing add inserts its zero under its type
	/// path, and the picker drops it from the options.
	#[beet_core::test]
	fn adding_inserts_the_zero_and_drops_the_option() {
		let (mut world, root) = build();
		let picker = test_ext::element_in(&mut world, "select");
		world
			.entity_mut(picker)
			.get_mut::<Value>()
			.unwrap()
			.set_if_neq(Value::str(Health::type_path()));
		let add = test_ext::collection_add(&mut world, "entities.0.components");
		test_ext::click_world(&mut world, add);
		SceneEntities::of(&test_ext::document_of(&mut world, root))
			.unwrap()
			.components(0)
			.unwrap()
			.clone()
			.xpect_eq(Map::new([
				(NAME, Value::str("a")),
				(Health::type_path(), Value::Uint(0)),
			]));
		test_ext::render_world(&mut world, root)
			.xnot()
			.xpect_contains(">Health</option>");
	}
}
