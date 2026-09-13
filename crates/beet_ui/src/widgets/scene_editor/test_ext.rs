//! Harness for the scene editor tests: a page authored with the editor tag,
//! forked into a scene document, its furniture bound and settled.
//!
//! Layered on the widget harnesses ([`test_ext`](crate::widgets::test_ext),
//! [`schema_ui::test_ext`](crate::widgets::schema_ui::test_ext)), which it
//! re-exports, so a scene editor test names one harness.
pub(super) use crate::widgets::schema_ui::test_ext::*;

use super::InspectorAction;
use super::SceneTreeRow;
use crate::prelude::*;
use crate::widgets::schema_ui::collection_edit::CollectionButton;
use crate::widgets::schema_ui::collection_edit::CollectionEdit;
use crate::widgets::schema_ui::collection_edit::NewEntryKey;
use beet_core::prelude::*;

/// The page every test edits: an element tree with an attribute, a text node
/// and the editor tag, keyed `0..=5` in fork order.
const PAGE: &str =
	"<main><h1>Garden</h1><p class=\"note\"></p><ToggleSceneEditor/></main>";

/// A world with the widget set, the charcell renderer and the material rules,
/// so a test renders the editor as the terminal paints it.
fn editor_world() -> World {
	(
		TemplatePlugin,
		MinimalTypesPlugin,
		DocumentPlugin,
		BsxDefaultsPlugin,
		FormPlugin,
		CharcellPlugin,
		crate::style::material::MaterialStylePlugin::default(),
	)
		.into_world()
}

/// [`PAGE`] built under a host and forked into its scene document, with the
/// editor opened, bound and every widget settled: `(world, host)`.
pub(super) fn editor() -> (World, Entity) {
	let mut world = editor_world();
	let host = world.spawn_empty().id();
	TemplateLoader::new(&mut world)
		.with_entity(host)
		.load(&MediaBytes::new_bsx(PAGE))
		.unwrap();
	SceneDocument::fork(&mut world, host, MediaType::Json).unwrap();
	// the disclosure ships closed; every test wants the editor showing
	let details = element_in(&mut world, "details");
	world.spawn((AttributeOf::new(details), Attribute::new("open")));
	settle_world(&mut world);
	settle_world(&mut world);
	(world, host)
}

/// Render `world`'s whole tree to plain charcell text, 100 columns wide.
pub(super) fn render_charcell(world: &mut World) -> String {
	let root = world
		.query_filtered::<Entity, (Without<ChildOf>, With<Children>)>()
		.iter(world)
		.next()
		.expect("no root");
	world.entity_mut(root).insert(FlexBuffer::new(100));
	world.run_schedule(crate::parse::PostParseTree);
	world
		.entity_mut(root)
		.take::<FlexBuffer>()
		.unwrap()
		.render_plain()
}

/// The tree row standing for the entity at `key`.
pub(super) fn row(world: &mut World, key: u32) -> Entity {
	world
		.query::<(Entity, &SceneTreeRow)>()
		.iter(world)
		.find(|(_, row)| row.key == key)
		.map(|(entity, _)| entity)
		.unwrap_or_else(|| panic!("no tree row for entity #{key}"))
}

/// Select the entity at `key` by activating its row, then settle.
pub(super) fn select(world: &mut World, key: u32) {
	let row = row(world, key);
	click_world(world, row);
	settle_world(world);
}

/// The live entity the scene's `key` built into.
pub(super) fn live(world: &World, host: Entity, key: u32) -> Entity {
	world
		.get::<TemplateEntityMap>(host)
		.unwrap()
		.world(key)
		.unwrap_or_else(|| panic!("entity #{key} is not live"))
}

/// The inspector's component picker, the keyed map's add-entry select.
pub(super) fn component_picker(world: &mut World) -> Entity {
	world
		.query_filtered::<(Entity, &Element), With<NewEntryKey>>()
		.iter(world)
		.find(|(_, element)| element.tag() == "select")
		.map(|(entity, _)| entity)
		.expect("no component picker")
}

/// The remove button of the component card keyed `type_path`.
pub(super) fn remove_button(world: &mut World, type_path: &str) -> Entity {
	world
		.query::<(Entity, &CollectionButton)>()
		.iter(world)
		.find(|(_, button)| {
			matches!(&button.edit, CollectionEdit::Remove(FieldSegment::ObjectKey(key)) if key == type_path)
		})
		.map(|(entity, _)| entity)
		.unwrap_or_else(|| panic!("no remove button for `{type_path}`"))
}

/// The inspector header's button for `action`.
pub(super) fn header_button(
	world: &mut World,
	action: InspectorAction,
) -> Entity {
	world
		.query::<(Entity, &InspectorAction)>()
		.iter(world)
		.find(|(_, found)| **found == action)
		.map(|(entity, _)| entity)
		.unwrap_or_else(|| panic!("no header button for {action:?}"))
}

/// Edit the scene document from outside the editor, then settle.
pub(super) fn edit(
	world: &mut World,
	host: Entity,
	func: impl FnOnce(&mut Value),
) {
	func(&mut world.get_mut::<Document>(host).unwrap().0);
	settle_world(world);
}
