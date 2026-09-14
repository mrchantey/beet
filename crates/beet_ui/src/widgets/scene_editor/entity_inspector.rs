//! [`EntityInspector`]: the selected entity as the list of components it is.
use super::SceneQuery;
use super::SceneSelection;
use super::style::*;
use crate::prelude::Button;
use crate::prelude::*;
use crate::widgets::schema_ui::field_layout::empty_note;
use crate::widgets::schema_ui::value_rebuild::RebuildKey;
use crate::widgets::schema_ui::value_rebuild::ValueRebuild;
use beet_core::prelude::*;

/// The entity the scene's [`SceneSelection`] names, as a form over the keyed
/// map it is ([`MapSchema::Keyed`]) at its document path.
///
/// An entity is a list of components, never a struct of a million optionals,
/// so the inspector is the form's map arm specialized twice: each component
/// is typed by the schema registered under its key, the add-entry key is a
/// component picker over the registry, and remove is the per-entry button the
/// map arm already has. A `ChildOf` entry's control is the entity picker, so
/// reparenting is an edit of that field. Above the form, the entity's label
/// and the two structural edits the form cannot express: a child added under
/// it, and the entity removed with its subtree.
///
/// The form is one generation per selection, respawned when the selection
/// changes and left alone while it does not, so typing into it is never
/// interrupted by the edit it makes.
#[template]
pub fn EntityInspector() -> impl Bundle {
	rsx! {
		<div {(InspectorHolder::default(), Classes::new([SCENE_INSPECTOR]))}/>
	}
}

/// Marks an [`EntityInspector`], remembering what its generation shows.
#[derive(Debug, Default, Component)]
pub(super) struct InspectorHolder {
	shown: Shown,
}

/// What an inspector's generation was built for.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Shown {
	#[default]
	Nothing,
	/// No scene is bound yet.
	NoScene,
	/// A scene with nothing selected.
	NoSelection,
	/// The entity at this key.
	Entity(u32),
}

/// The structural edits the inspector's header offers, which the form over
/// one entity cannot express.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
#[component(on_add = hook_ext::observe(apply_inspector_action))]
pub(super) enum InspectorAction {
	/// Spawn a new entity under the selected one and select it.
	AddChild,
	/// Remove the selected entity and everything it owns, selecting its
	/// parent.
	Remove,
}

/// System: regenerate each inspector whose selection changed.
pub(super) fn rebuild_inspectors(
	mut holders: Query<(Entity, &mut InspectorHolder, Option<&Children>)>,
	scene: SceneQuery,
	selections: Query<&SceneSelection>,
	mut commands: Commands,
) {
	for (entity, mut holder, children) in holders.iter_mut() {
		let selection = scene
			.host(entity)
			.and_then(|host| selections.get(host).ok());
		let wanted = match selection {
			None => Shown::NoScene,
			Some(SceneSelection(None)) => Shown::NoSelection,
			Some(SceneSelection(Some(key))) => Shown::Entity(*key),
		};
		if holder.shown == wanted {
			continue;
		}
		holder.shown = wanted;
		for child in children.into_iter().flat_map(|children| children.iter()) {
			commands.entity(child).despawn();
		}
		commands
			.spawn(ChildOf(entity))
			.insert_template(generation(wanted));
	}
}

/// One generation of the inspector.
fn generation(shown: Shown) -> Snippet {
	match shown {
		Shown::Nothing | Shown::NoScene => {
			empty_note("No scene document to edit")
		}
		Shown::NoSelection => empty_note("Select an entity in the tree"),
		Shown::Entity(key) => {
			let field = FieldRef::new(SceneEntities::entity_path(key));
			rsx! {
				<div {Classes::new([SCENE_INSPECTOR_HEADER])}>
					{title(key)}
					<div {Classes::new([SCENE_INSPECTOR_ACTIONS])}>
						<Button
							action=true
							variant={ButtonVariant::Tonal}
							{InspectorAction::AddChild}
						>"Add child"</Button>
						<Button
							action=true
							variant={ButtonVariant::Text}
							{InspectorAction::Remove}
						>"Remove"</Button>
					</div>
				</div>
				<DynamicForm
					schema={ValueSchema::Map(MapSchema::Keyed)}
					field={field}
				/>
			}
			.any_snippet()
		}
	}
}

/// The entity's label as the tree shows it, riding its own rebuild keyed on
/// the label so a rename reaches the title without regenerating the form
/// being typed into.
fn title(key: u32) -> Snippet {
	let rebuild = ValueRebuild::new(
		move |scene| vec![RebuildKey::Name(label(scene, key).into())],
		move |_resolver, scene, _key| {
			let label = label(scene, key);
			rsx! { <span {Classes::new([SCENE_INSPECTOR_TITLE])}>{label}</span> }
				.any_snippet()
		},
	);
	rsx! { <span {(FieldRef::default(), rebuild)}/> }.any_snippet()
}

/// The label of the entity at `key` in `scene`, or the key alone while the
/// document has not synced.
fn label(scene: &Value, key: u32) -> String {
	SceneEntities::of(scene)
		.map(|entities| entities.label(key))
		.unwrap_or_else(|_| key.to_string())
}

/// Observer: a header button applies its structural edit to the scene
/// document and moves the selection where the edit leaves it.
fn apply_inspector_action(
	ev: On<PointerUp>,
	actions: Query<&InspectorAction>,
	scene: SceneQuery,
	mut selections: Query<&mut SceneSelection>,
	mut docs: DocumentQuery,
) -> Result {
	// the event bubbles; act only at the button itself
	let target = ev.event_target();
	let Ok(action) = actions.get(target) else {
		return OK;
	};
	let Some(host) = scene.host(target) else {
		return OK;
	};
	let Some(selected) = selections.get(host)?.0 else {
		return OK;
	};
	let action = *action;
	let next =
		docs.with_field(target, &FieldRef::default(), |scene| match action {
			InspectorAction::AddChild => add_child(scene, selected),
			InspectorAction::Remove => remove(scene, selected),
		})??;
	selections.get_mut(host)?.set_if_neq(SceneSelection(next));
	OK
}

/// Add an entity under `parent`, keyed above every key the scene holds, and
/// answer its key for selection.
fn add_child(scene: &mut Value, parent: u32) -> Result<Option<u32>> {
	let key = SceneEntities::of(scene)?
		.keys()?
		.into_iter()
		.max()
		.map(|key| key + 1)
		.unwrap_or_default();
	SceneEntities::of_mut(scene)?.insert_entity(
		key,
		Map::new([(ChildOf::type_path(), EntitySchema::reference(parent)?)]),
	);
	Some(key).xok()
}

/// Remove `key` and every entity it owns, transitively, answering the parent
/// it sat under for selection.
fn remove(scene: &mut Value, key: u32) -> Result<Option<u32>> {
	let entities = SceneEntities::of(scene)?;
	let parent = entities.target(key, ChildOf::type_path());
	// an owner is a parent or, for an attribute entity, its element
	let owner = |key: u32| {
		entities
			.target(key, ChildOf::type_path())
			.or_else(|| entities.target(key, AttributeOf::type_path()))
	};
	let mut removed = vec![key];
	let mut frontier = vec![key];
	while let Some(current) = frontier.pop() {
		for owned in entities
			.keys()?
			.into_iter()
			.filter(|candidate| owner(*candidate) == Some(current))
		{
			removed.push(owned);
			frontier.push(owned);
		}
	}
	let mut entities = SceneEntities::of_mut(scene)?;
	for key in removed {
		entities.remove_entity(key);
	}
	Ok(parent)
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::widgets::scene_editor::test_ext;
	use crate::widgets::schema_ui::test_ext as schema_test_ext;

	const CHILD_OF: &str = "bevy_ecs::hierarchy::ChildOf";

	/// Selecting an entity generates a form over its components, one card per
	/// component typed by the schema its key names.
	#[beet_core::test]
	fn selection_generates_the_component_form() {
		let (mut world, host) = test_ext::editor();
		test_ext::render_world(&mut world, host)
			.xpect_contains("Select an entity in the tree");
		test_ext::select(&mut world, 1);
		let html = test_ext::render_world(&mut world, host);
		html.clone()
			.xpect_contains(">#1 h1</span>")
			// the element's tag is a text control, its parent an entity picker
			.xpect_contains(
				"name=\"entities.1.beet_core::types::element::element::Element\"",
			)
			.xpect_contains(
				"<select name=\"entities.1.bevy_ecs::hierarchy::ChildOf\"",
			)
			.xpect_contains("Add component");
		html.xpect_contains("<label>Element<input");
	}

	/// Typing into a component's control edits the document, and the world
	/// follows in place: item 3's live edit.
	#[beet_core::test]
	fn an_edit_reaches_the_document_and_the_world() {
		let (mut world, host) = test_ext::editor();
		test_ext::select(&mut world, 2);
		let control = schema_test_ext::bound(
			&mut world,
			"entities.2.beet_core::types::value::value::Value",
		);
		world
			.entity_mut(control)
			.get_mut::<Value>()
			.unwrap()
			.set_if_neq(Value::str("Orchard"));
		test_ext::settle_world(&mut world);
		let text = test_ext::live(&world, host, 2);
		world
			.get::<Value>(text)
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("Orchard");
		// the tree's row followed the rename
		test_ext::render_charcell(&mut world).xpect_contains("\"Orchard\"");
	}

	/// Adding a component through the picker inserts its zero, which the world
	/// applies, and removing it through its card drops it again.
	#[beet_core::test]
	fn a_component_is_added_and_removed() {
		let (mut world, host) = test_ext::editor();
		test_ext::select(&mut world, 3);
		let paragraph = test_ext::live(&world, host, 3);
		let picker = test_ext::component_picker(&mut world);
		world
			.entity_mut(picker)
			.get_mut::<Value>()
			.unwrap()
			.set_if_neq(Value::str(Name::type_path()));
		let add = schema_test_ext::collection_add(&mut world, "entities.3");
		test_ext::click_world(&mut world, add);
		world.get::<Name>(paragraph).unwrap().as_str().xpect_eq("");
		test_ext::render_world(&mut world, host)
			.xpect_contains("<label>Name<input");

		let remove = test_ext::remove_button(&mut world, Name::type_path());
		test_ext::click_world(&mut world, remove);
		world.get::<Name>(paragraph).xpect_none();
		test_ext::render_world(&mut world, host)
			.xnot()
			.xpect_contains("<label>Name<input");
	}

	/// Choosing another parent in the `ChildOf` picker reparents the live
	/// entity, and the tree redraws it under its new parent.
	#[beet_core::test]
	fn a_reparent_moves_the_entity() {
		let (mut world, host) = test_ext::editor();
		test_ext::select(&mut world, 3);
		let (heading, paragraph) = (
			test_ext::live(&world, host, 1),
			test_ext::live(&world, host, 3),
		);
		let picker = schema_test_ext::element_in(&mut world, "select");
		world
			.entity_mut(picker)
			.get_mut::<Value>()
			.unwrap()
			.set_if_neq(Value::str("1"));
		test_ext::settle_world(&mut world);
		world
			.get::<ChildOf>(paragraph)
			.unwrap()
			.parent()
			.xpect_eq(heading);
		SceneEntities::of(&test_ext::document_of(&mut world, host))
			.unwrap()
			.target(3, CHILD_OF)
			.unwrap()
			.xpect_eq(1);
		test_ext::render_charcell(&mut world)
			.xpect_contains("│ ├ #2 \"Garden\"")
			.xpect_contains("│ └ #3 p");
	}

	/// The header's structural edits: a child is added under the selection
	/// and selected, and a removal takes the subtree and selects the parent.
	#[beet_core::test]
	fn a_child_is_added_and_a_subtree_removed() {
		let (mut world, host) = test_ext::editor();
		test_ext::select(&mut world, 3);
		let add =
			test_ext::header_button(&mut world, InspectorAction::AddChild);
		test_ext::click_world(&mut world, add);
		world
			.get::<SceneSelection>(host)
			.unwrap()
			.xpect_eq(SceneSelection(Some(6)));
		let paragraph = test_ext::live(&world, host, 3);
		let child = test_ext::live(&world, host, 6);
		world
			.get::<ChildOf>(child)
			.unwrap()
			.parent()
			.xpect_eq(paragraph);

		test_ext::select(&mut world, 3);
		let remove =
			test_ext::header_button(&mut world, InspectorAction::Remove);
		test_ext::click_world(&mut world, remove);
		world
			.get::<SceneSelection>(host)
			.unwrap()
			.xpect_eq(SceneSelection(Some(0)));
		world.get_entity(paragraph).is_err().xpect_true();
		world.get_entity(child).is_err().xpect_true();
		let scene = test_ext::document_of(&mut world, host);
		let entities = SceneEntities::of(&scene).unwrap();
		entities.contains(3).xpect_false();
		entities.contains(4).xpect_false();
		entities.contains(6).xpect_false();
	}
}
