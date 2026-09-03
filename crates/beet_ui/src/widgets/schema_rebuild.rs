//! [`SchemaRebuild`]: regenerating a schema-generated subtree when the schema
//! it was generated from changes.
//!
//! [`DynamicForm`] and [`DynamicView`] read their schema once and emit a tree.
//! The values their leaves bind are reactive, and a view's rows are, but the
//! schema is not: a committed schema edit adds a column only if the subtree
//! generated from it is rebuilt. This is that rebuild, and the one place a
//! schema-driven widget's reactivity to its *schema* lives.
use beet_core::prelude::*;
use bevy::platform::sync::Arc;

/// Holds the schema a subtree was generated from and the builder that
/// regenerates it, so a schema edit landing in the [`SchemaRegistry`] respawns
/// every subtree that schema describes.
///
/// The holder is **element-less**, so it adds a transparent node rather than a
/// wrapper: both renderers pass through a node with no `Element`. Its children
/// are one generation and nothing else, so a rebuild despawns all of them,
/// while a `<Slot/>`'s authored children (a form's submit button) are its
/// siblings and survive untouched.
///
/// A rebuild fires exactly when the schema the subtree renders *changed*: the
/// authored schema is re-resolved against the registry on every registry change
/// and compared with what the current generation was built from. A schema
/// naming no [`ValueSchema::Reference`] resolves to itself and so never
/// rebuilds; a reference that was still arriving at build time rebuilds the
/// moment it lands.
#[derive(Component)]
pub struct SchemaRebuild {
	/// The authored schema, re-resolved on every registry change.
	schema: ValueSchema,
	/// The fully resolved schema the current generation was built from, the
	/// fingerprint a rebuild is decided by.
	resolved: ValueSchema,
	/// Builds one generation.
	build: Arc<dyn for<'a> Fn(SchemaResolver<'a>) -> Snippet + Send + Sync>,
}

impl SchemaRebuild {
	/// Hold `schema` and the `build` that renders it, fingerprinted against
	/// `resolver` as it stands now.
	pub fn new(
		resolver: SchemaResolver,
		schema: ValueSchema,
		build: impl 'static
		+ Send
		+ Sync
		+ for<'a> Fn(SchemaResolver<'a>) -> Snippet,
	) -> Self {
		Self {
			resolved: Self::resolve(resolver, &schema),
			schema,
			build: Arc::new(build),
		}
	}

	/// The holder node a schema-driven template emits in place of its generated
	/// subtree: this rebuild, carrying the first generation as its only child.
	pub fn holder(self, resolver: SchemaResolver) -> Snippet {
		let generation = (self.build)(resolver);
		Snippet::from_bundle((self, children![generation]))
	}

	/// The schema as `resolver` currently resolves it, or the schema itself when
	/// there is no registry to resolve against.
	fn resolve(resolver: SchemaResolver, schema: &ValueSchema) -> ValueSchema {
		resolver
			.registry()
			.map(|registry| registry.resolve(schema))
			.unwrap_or_else(|| schema.clone())
	}
}

/// Respawn the generation of every [`SchemaRebuild`] whose schema a registry
/// edit changed, leaving the rest (and every holder's siblings) alone.
pub(super) fn rebuild_schema_widgets(
	registry: Res<SchemaRegistry>,
	mut holders: Populated<(Entity, &mut SchemaRebuild, Option<&Children>)>,
	mut commands: Commands,
) {
	let resolver = SchemaResolver::default().with_schemas(&registry);
	for (entity, mut rebuild, children) in holders.iter_mut() {
		let resolved = SchemaRebuild::resolve(resolver, &rebuild.schema);
		if resolved == rebuild.resolved {
			continue;
		}
		rebuild.resolved = resolved;
		if let Some(children) = children {
			for child in children.iter() {
				commands.entity(child).despawn();
			}
		}
		commands.spawn((ChildOf(entity), (rebuild.build)(resolver)));
	}
}

#[cfg(test)]
mod test {
	use super::super::test_ext;
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[derive(Reflect)]
	struct Profile {
		name: String,
	}

	/// A form over a named schema regenerates its controls when that schema is
	/// edited: the rebuild is what makes item 3's "the form gains a field" true
	/// of a form that was built before the edit.
	#[beet_core::test]
	fn a_registry_edit_regenerates_the_form() {
		let mut world = world_ext::ui_world();
		world
			.get_resource_or_init::<SchemaRegistry>()
			.insert("Profile", ValueSchema::of::<Profile>());
		let root = world
			.spawn_template(rsx! {
				<DynamicForm schema={ValueSchema::Reference("Profile".into())}/>
			})
			.unwrap()
			.id();
		world.update_local();
		test_ext::render_world(&mut world, root)
			.xnot()
			.xpect_contains("type=\"checkbox\"");

		world.get_resource_or_init::<SchemaRegistry>().insert(
			"Profile",
			ValueSchema::Struct(StructSchema {
				name: Some("Profile".into()),
				allow_additional: false,
				fields: vec![
					NamedFieldSchema::new(
						"name",
						ValueSchema::String(default()),
					),
					NamedFieldSchema::new("done", ValueSchema::Bool(default())),
				],
			}),
		);
		world.update_local();

		test_ext::render_world(&mut world, root)
			.xpect_contains("type=\"text\"")
			.xpect_contains("type=\"checkbox\"");
	}

	/// The rebuild replaces exactly one generation: the previous controls are
	/// gone rather than accumulating, so a form does not grow a duplicate field
	/// per edit.
	#[beet_core::test]
	fn a_rebuild_replaces_rather_than_appends() {
		let mut world = world_ext::ui_world();
		world
			.get_resource_or_init::<SchemaRegistry>()
			.insert("Profile", ValueSchema::of::<Profile>());
		world
			.spawn_template(rsx! {
				<DynamicForm schema={ValueSchema::Reference("Profile".into())}/>
			})
			.unwrap();
		world.update_local();
		world
			.get_resource_or_init::<SchemaRegistry>()
			.insert("Profile", ValueSchema::of::<Profile>());
		world.update_local();

		world
			.query_once::<(&Element, &FieldRef)>()
			.into_iter()
			.filter(|(element, _)| element.tag() == "input")
			.count()
			// the identical re-registration resolves the same, so nothing rebuilt
			.xpect_eq(1);
	}

	/// A form's slotted children are the holder's siblings, so a rebuild leaves
	/// them in place — and in place *after* the regenerated controls.
	#[beet_core::test]
	fn a_rebuild_keeps_the_slot() {
		let mut world = world_ext::ui_world();
		world
			.get_resource_or_init::<SchemaRegistry>()
			.insert("Profile", ValueSchema::of::<Profile>());
		let root = world
			.spawn_template(rsx! {
				<DynamicForm schema={ValueSchema::Reference("Profile".into())}>
					<Button>"Save"</Button>
				</DynamicForm>
			})
			.unwrap()
			.id();
		world.update_local();
		world.get_resource_or_init::<SchemaRegistry>().insert(
			"Profile",
			ValueSchema::Struct(StructSchema {
				name: Some("Profile".into()),
				allow_additional: false,
				fields: vec![NamedFieldSchema::new(
					"done",
					ValueSchema::Bool(default()),
				)],
			}),
		);
		world.update_local();

		let html = test_ext::render_world(&mut world, root);
		html.clone()
			.xpect_contains("type=\"checkbox\"")
			.xpect_contains("Save");
		// the regenerated controls still precede the slot content
		html.find("checkbox")
			.unwrap()
			.lt(&html.find("Save").unwrap())
			.xpect_true();
	}
}
