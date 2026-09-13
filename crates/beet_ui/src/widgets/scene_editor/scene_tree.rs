//! [`SceneTree`]: the scene document's hierarchy, one selectable row per
//! entity.
use super::SceneQuery;
use super::SceneSelection;
use super::style::*;
use crate::prelude::*;
use crate::widgets::schema_ui::field_layout::empty_note;
use crate::widgets::schema_ui::value_rebuild::RebuildKey;
use crate::widgets::schema_ui::value_rebuild::ValueRebuild;
use beet_core::prelude::*;

/// The hierarchy of the scene document this widget binds into: one focusable
/// row per entity in document order, nested under its parent (a `ChildOf`) or
/// its element (an `AttributeOf`) with tree guides, labelled by name else by
/// key and kind ([`SceneEntities::label`]). Activating a row selects its
/// entity ([`SceneSelection`]), which the [`EntityInspector`] beside it
/// follows.
///
/// The rows ride a [`ValueRebuild`] over the whole document keyed by each
/// entity's key, place and label, so an entity added, moved, renamed or
/// removed rebuilds its own row alone and every other row keeps its entity
/// and focus. The editor's furniture never appears: it is [`Derived`], so it
/// is not in the document the tree reads.
///
/// [`EntityInspector`]: super::EntityInspector
#[template]
pub fn SceneTree() -> impl Bundle {
	let rebuild = ValueRebuild::new(
		|scene| match rows(scene) {
			rows if rows.is_empty() => vec![RebuildKey::Empty],
			rows => rows.into_iter().map(|row| row.rebuild_key()).collect(),
		},
		|_resolver, scene, key| match key {
			RebuildKey::Name(_) => rows(scene)
				.into_iter()
				.find(|row| row.rebuild_key() == *key)
				.map(|row| row.build())
				.unwrap_or_else(|| Snippet::from_bundle(())),
			_ if SceneEntities::of(scene).is_ok() => {
				empty_note("No entities in this scene")
			}
			_ => empty_note("No scene document to edit"),
		},
	);
	rsx! {
		<div {(FieldRef::default(), rebuild, Classes::new([SCENE_TREE]))}/>
	}
}

/// Marks a [`SceneTree`] row, carrying the file key of the entity it stands
/// for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
#[component(on_add = hook_ext::observe(select_row_on_activate))]
pub struct SceneTreeRow {
	/// The entity's file key.
	pub key: u32,
}

/// One row of the tree as the document describes it.
struct TreeRow {
	key: u32,
	/// The guide glyphs that draw the row's place in the tree.
	guides: String,
	label: String,
}

impl TreeRow {
	/// The identity a row keeps across edits: its entity, its place and its
	/// label, so a change to any of them rebuilds the row and nothing else does.
	fn rebuild_key(&self) -> RebuildKey {
		RebuildKey::Name(
			format!("{}|{}|{}", self.key, self.guides, self.label).into(),
		)
	}

	fn build(self) -> Snippet {
		let Self { key, guides, label } = self;
		rsx! {
			<div
				{(SceneTreeRow { key }, focusable(), Classes::new([SCENE_TREE_ROW]))}
				tabindex="0"
			>
				<span {Classes::new([SCENE_TREE_GUIDES])}>{guides}</span>
				{label}
			</div>
		}
		.any_snippet()
	}
}

/// What makes a row reachable by Tab where the focus model exists; the web
/// reaches it through the row's `tabindex`.
#[cfg(feature = "keyboard")]
fn focusable() -> impl Bundle { Focusable }
#[cfg(not(feature = "keyboard"))]
fn focusable() -> impl Bundle {}

/// The tree's rows in display order: every entity of `scene`, depth first
/// under the entity that owns it, each with the guides that draw its place.
fn rows(scene: &Value) -> Vec<TreeRow> {
	let Ok(entities) = SceneEntities::of(scene) else {
		return Vec::new();
	};
	let keys = entities.keys().unwrap_or_default();
	// what owns each entity: its parent, else the element an attribute
	// belongs to, else nothing (a root), an owner outside the scene counting
	// as nothing rather than as a hole in the tree
	let owner = |key: u32| {
		entities
			.target(key, ChildOf::type_path())
			.or_else(|| entities.target(key, AttributeOf::type_path()))
			.filter(|owner| entities.contains(*owner))
	};
	let mut children = HashMap::<Option<u32>, Vec<u32>>::default();
	for key in &keys {
		children.entry(owner(*key)).or_default().push(*key);
	}
	let mut rows = Vec::with_capacity(keys.len());
	// a root draws no guides; below it each frame carries whether every
	// ancestor level under the root, and the row itself, has a later sibling,
	// which is what its guide column draws
	let mut stack = children
		.remove(&None)
		.unwrap_or_default()
		.into_iter()
		.rev()
		.map(|key| (key, None::<(Vec<bool>, bool)>))
		.collect::<Vec<_>>();
	while let Some((key, place)) = stack.pop() {
		let guides = place
			.as_ref()
			.map(|(ancestors, has_next)| {
				ancestors
					.iter()
					.map(|open| match open {
						true => "│\u{a0}",
						false => "\u{a0}\u{a0}",
					})
					.chain([match has_next {
						true => "├\u{a0}",
						false => "└\u{a0}",
					}])
					.collect::<String>()
			})
			.unwrap_or_default();
		rows.push(TreeRow {
			key,
			guides,
			label: entities.label(key),
		});
		let lineage = place
			.map(|(mut ancestors, has_next)| {
				ancestors.push(has_next);
				ancestors
			})
			.unwrap_or_default();
		let kids = children.remove(&Some(key)).unwrap_or_default();
		let last = kids.len().saturating_sub(1);
		for (index, kid) in kids.into_iter().enumerate().rev() {
			stack.push((kid, Some((lineage.clone(), index != last))));
		}
	}
	rows
}

/// Observer: activating a row (a click, or Enter while focused) selects its
/// entity on the scene the editor is bound to.
fn select_row_on_activate(
	ev: On<PointerUp>,
	rows: Query<&SceneTreeRow>,
	scene: SceneQuery,
	mut selections: Query<&mut SceneSelection>,
) -> Result {
	// the event bubbles; act only at the row itself
	let target = ev.event_target();
	let Ok(row) = rows.get(target) else {
		return OK;
	};
	let Some(host) = scene.host(target) else {
		return OK;
	};
	selections
		.get_mut(host)?
		.set_if_neq(SceneSelection(Some(row.key)));
	OK
}

/// System: mark each row `aria-selected` exactly when its entity is the
/// scene's selection, the attribute both targets style by.
///
/// Runs when a selection changes or a row arrives, so a rebuilt row (its
/// entity renamed, say) wears the state its predecessor did.
pub(super) fn sync_tree_selection(
	rows: Query<(Entity, &SceneTreeRow)>,
	added: Query<(), Added<SceneTreeRow>>,
	scene: SceneQuery,
	selections: Query<Ref<SceneSelection>>,
	attributes: Query<&Attributes>,
	attr_keys: Query<&Attribute>,
	mut values: Query<&mut Value>,
	mut states: Query<&mut ElementStateMap>,
	mut commands: Commands,
) {
	for (entity, row) in rows.iter() {
		let Some(selection) = scene
			.host(entity)
			.and_then(|host| selections.get(host).ok())
		else {
			continue;
		};
		if !added.contains(entity) && !selection.is_changed() {
			continue;
		}
		let selected = selection.0 == Some(row.key);
		set_attr_str(
			&mut commands,
			&mut values,
			&mut states,
			&attributes,
			&attr_keys,
			entity,
			"aria-selected",
			if selected { "true" } else { "false" },
		);
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::widgets::scene_editor::test_ext;

	/// Every entity has a row, nested under what owns it, guides drawing the
	/// tree and labels naming what each is.
	#[beet_core::test]
	fn rows_follow_the_hierarchy() {
		let (mut world, _) = test_ext::editor();
		test_ext::render_charcell(&mut world)
			.xpect_contains("#0 main")
			.xpect_contains("├ #1 h1")
			.xpect_contains("│ └ #2 \"Garden\"")
			.xpect_contains("├ #3 p")
			.xpect_contains("│ └ #4 @class")
			.xpect_contains("└ #5 ToggleSceneEditor");
	}

	/// Activating a row selects its entity on the scene's host, and the row
	/// alone wears the selected state.
	#[beet_core::test]
	fn activating_a_row_selects_its_entity() {
		let (mut world, host) = test_ext::editor();
		let row = test_ext::row(&mut world, 3);
		test_ext::click_world(&mut world, row);
		world
			.get::<SceneSelection>(host)
			.unwrap()
			.xpect_eq(SceneSelection(Some(3)));
		let html = test_ext::render_world(&mut world, host);
		html.matches("aria-selected=\"true\"").count().xpect_eq(1);
		test_ext::row(&mut world, 1).xpect_not_eq(row);

		let row = test_ext::row(&mut world, 1);
		test_ext::click_world(&mut world, row);
		world
			.get::<SceneSelection>(host)
			.unwrap()
			.xpect_eq(SceneSelection(Some(1)));
	}

	/// A selected entity removed from outside the editor clears the selection,
	/// so the inspector shows nothing rather than a form over a missing entity.
	#[beet_core::test]
	fn a_removed_selection_clears() {
		let (mut world, host) = test_ext::editor();
		test_ext::select(&mut world, 4);
		test_ext::edit(&mut world, host, |scene| {
			scene
				.get_mut("entities")
				.unwrap()
				.as_map_mut()
				.unwrap()
				.remove("4");
		});
		world
			.get::<SceneSelection>(host)
			.unwrap()
			.xpect_eq(SceneSelection(None));
		test_ext::render_world(&mut world, host)
			.xpect_contains("Select an entity in the tree");
	}

	/// An edit elsewhere rebuilds only the row it changed: renaming the
	/// heading keeps every other row's entity, and the renamed row's state.
	#[beet_core::test]
	fn an_edit_rebuilds_only_its_row() {
		let (mut world, host) = test_ext::editor();
		let row = test_ext::row(&mut world, 1);
		test_ext::click_world(&mut world, row);
		let (heading, paragraph) =
			(test_ext::row(&mut world, 1), test_ext::row(&mut world, 3));
		test_ext::edit(&mut world, host, |scene| {
			scene
				.get_mut("entities")
				.unwrap()
				.get_mut("1")
				.unwrap()
				.get_mut("components")
				.unwrap()
				.insert(Name::type_path(), "Heading")
				.unwrap();
		});
		test_ext::row(&mut world, 3).xpect_eq(paragraph);
		test_ext::row(&mut world, 1).xpect_not_eq(heading);
		test_ext::render_charcell(&mut world).xpect_contains("├ Heading");
		// the rebuilt row is still the selected one
		test_ext::render_world(&mut world, host)
			.matches("aria-selected=\"true\"")
			.count()
			.xpect_eq(1);
	}
}
