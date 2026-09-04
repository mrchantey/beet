//! [`SchemaRebuild`]: regenerating a schema-generated subtree when the schema
//! it was generated from changes.
//!
//! [`DynamicForm`] and [`DynamicView`] read their schema once and emit a tree.
//! The values their leaves bind are reactive, and a view's rows are, but the
//! schema is not: a committed schema edit adds a column only if the subtree
//! generated from it is rebuilt. This is that rebuild, and the one place a
//! schema-driven widget's reactivity to its *schema* lives.
//!
//! A schema arrives two ways ([`SchemaSource`]): authored on the widget, or
//! declared by the document it binds into. Both are re-read here every time
//! something that could change them changes, so a widget mounted over a document
//! still being read out of a store generates itself the moment it lands.
use beet_core::prelude::*;
use bevy::platform::sync::Arc;

/// Where a schema-driven widget's schema comes from.
///
/// One or the other, never both: a widget that names a schema means it, and a
/// widget that names none takes the document's, since a document is the one
/// authority on the shape of its own value. Restating it on the widget would be
/// a second copy nothing keeps in step, which is the rule the bindings already
/// follow.
#[derive(Clone)]
pub enum SchemaSource {
	/// The schema authored on the widget.
	Authored(ValueSchema),
	/// The schema the bound document declares at `field`, absent until the
	/// document arrives.
	Document(FieldRef),
}

impl SchemaSource {
	/// The authored schema, or `None` for a document source, whose schema only
	/// the world can answer.
	fn authored(&self) -> Option<&ValueSchema> {
		match self {
			Self::Authored(schema) => Some(schema),
			Self::Document(_) => None,
		}
	}
}

/// Holds the schema a subtree was generated from and the builder that
/// regenerates it, so a schema edit landing in the [`SchemaRegistry`] — or in
/// the bound document — respawns every subtree that schema describes.
///
/// The holder is **element-less**, so it adds a transparent node rather than a
/// wrapper: both renderers pass through a node with no `Element`. Its children
/// are one generation and nothing else, so a rebuild despawns all of them,
/// while a `<Slot/>`'s authored children (a form's submit button) are its
/// siblings and survive untouched.
///
/// A rebuild fires exactly when the schema the subtree renders *changed*: the
/// source schema is re-resolved against the registry and compared with what the
/// current generation was built from. A schema naming no [`ValueSchema::Ref`]
/// resolves to itself and so never rebuilds; a reference that was still
/// arriving at build time rebuilds the moment it lands, and a document-sourced
/// widget has no generation at all until its document answers one.
#[derive(Component)]
pub struct SchemaRebuild {
	/// Where the schema comes from, re-read on every pass.
	source: SchemaSource,
	/// The fully resolved schema the current generation was built from, the
	/// fingerprint a rebuild is decided by. `None` before the first generation.
	resolved: Option<ValueSchema>,
	/// Builds one generation from the schema as its source currently reads it.
	build: Arc<
		dyn for<'a> Fn(SchemaResolver<'a>, &ValueSchema) -> Snippet
			+ Send
			+ Sync,
	>,
}

impl SchemaRebuild {
	/// Hold `source` and the `build` that renders it, fingerprinted against
	/// `resolver` as it stands now.
	pub fn new(
		resolver: SchemaResolver,
		source: SchemaSource,
		build: impl 'static
		+ Send
		+ Sync
		+ for<'a> Fn(SchemaResolver<'a>, &ValueSchema) -> Snippet,
	) -> Self {
		Self {
			resolved: source
				.authored()
				.map(|schema| Self::resolve(resolver, schema)),
			source,
			build: Arc::new(build),
		}
	}

	/// The holder node a schema-driven template emits in place of its generated
	/// subtree: this rebuild, carrying the first generation as its only child.
	///
	/// A document-sourced widget has no schema at build time, so it emits the
	/// holder alone and [`rebuild_schema_widgets`] spawns its first generation
	/// when the document answers one — the same shape `ValueRebuild` takes for
	/// a value that has not synced yet.
	pub fn holder(self, resolver: SchemaResolver) -> Snippet {
		match self
			.source
			.authored()
			.map(|schema| (self.build)(resolver, schema))
		{
			Some(generation) => {
				Snippet::from_bundle((self, children![generation]))
			}
			None => Snippet::from_bundle(self),
		}
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

/// Run condition for [`rebuild_schema_widgets`]: the three ways the schema a
/// holder renders can change — a registry edit, a document declaring a new
/// schema (or arriving with one), and a holder built after the document it
/// reads.
pub(super) fn schema_widgets_may_rebuild(
	registry: Option<Res<SchemaRegistry>>,
	changed_schemas: Query<(), Changed<DocumentSchema>>,
	new_holders: Query<(), Added<SchemaRebuild>>,
) -> bool {
	registry.is_some_and(|registry| registry.is_changed())
		|| !changed_schemas.is_empty()
		|| !new_holders.is_empty()
}

/// Respawn the generation of every [`SchemaRebuild`] whose schema changed,
/// leaving the rest (and every holder's siblings) alone.
pub(super) fn rebuild_schema_widgets(
	registry: Option<Res<SchemaRegistry>>,
	documents: DocumentQuery,
	mut holders: Populated<(Entity, &mut SchemaRebuild, Option<&Children>)>,
	mut commands: Commands,
) {
	let resolver = match registry.as_deref() {
		Some(registry) => SchemaResolver::default().with_schemas(registry),
		None => SchemaResolver::default(),
	};
	for (entity, mut rebuild, children) in holders.iter_mut() {
		// the source is re-read rather than remembered: a document's schema is
		// its own, and may arrive, change or be committed long after this holder
		let schema = match &rebuild.source {
			SchemaSource::Authored(schema) => Some(schema.clone()),
			SchemaSource::Document(field) => {
				documents.field_schema(entity, field)
			}
		};
		// a document that has not answered yet leaves the holder empty, which is
		// the still-arriving case rather than a failure
		let Some(schema) = schema else { continue };
		let resolved = SchemaRebuild::resolve(resolver, &schema);
		if rebuild.resolved.as_ref() == Some(&resolved) {
			continue;
		}
		rebuild.resolved = Some(resolved);
		if let Some(children) = children {
			for child in children.iter() {
				commands.entity(child).despawn();
			}
		}
		commands.spawn((ChildOf(entity), (rebuild.build)(resolver, &schema)));
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
				<DynamicForm schema={ValueSchema::reference("Profile")}/>
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
				<DynamicForm schema={ValueSchema::reference("Profile")}/>
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
				<DynamicForm schema={ValueSchema::reference("Profile")}>
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

	/// A rebuilt view keeps its rows: the regenerated subtree's own bindings
	/// must be seeded from the document that is already there, since nothing
	/// changes it afterwards to fan out to them.
	#[beet_core::test]
	fn a_rebuilt_view_keeps_its_rows() {
		let mut world = world_ext::ui_world();
		let item = |fields: Vec<NamedFieldSchema>| {
			ValueSchema::Struct(StructSchema {
				name: Some("TodoItem".into()),
				allow_additional: false,
				fields,
			})
		};
		let label =
			NamedFieldSchema::new("label", ValueSchema::String(default()));
		world
			.get_resource_or_init::<SchemaRegistry>()
			.insert("TodoItem", item(vec![label.clone()]));
		let root = world
			.spawn((
				Document::new(value!([{ "label": "buy milk" }])),
				DocumentSchema(ValueSchema::List(ListSchema {
					item: Box::new(ValueSchema::reference("TodoItem")),
					min_items: None,
					max_items: None,
					unique: false,
				})),
			))
			.id();
		let view = world.spawn_template(rsx! { <DynamicView/> }).unwrap().id();
		world.entity_mut(root).add_child(view);
		test_ext::settle_world(&mut world);
		test_ext::render_world(&mut world, root).xpect_contains("buy milk");

		world.get_resource_or_init::<SchemaRegistry>().insert(
			"TodoItem",
			item(vec![
				label,
				NamedFieldSchema::new("done", ValueSchema::Bool(default())),
			]),
		);
		test_ext::settle_world(&mut world);

		test_ext::render_world(&mut world, root)
			.xpect_contains("<th>done</th>")
			.xpect_contains("buy milk");
	}

	/// A widget naming no schema takes the one its document declares, and takes
	/// it whenever it lands: the store read that answers a document resolves
	/// frames after the tree that binds it is built, so a view mounted over one
	/// has no generation until then and a full one after.
	#[beet_core::test]
	fn a_document_schema_arriving_late_generates_the_view() {
		let mut world = world_ext::ui_world();
		let root = world.spawn_template(rsx! { <DynamicView/> }).unwrap().id();
		world.update_local();
		// nothing to read yet: the holder is there, its generation is not
		test_ext::render_world(&mut world, root)
			.xnot()
			.xpect_contains("name");

		world.entity_mut(root).insert((
			Document::new(value!({ "name": "buy milk" })),
			DocumentSchema::of::<Profile>(),
		));
		test_ext::settle_world(&mut world);

		test_ext::render_world(&mut world, root)
			.xpect_contains("name")
			.xpect_contains("buy milk");
	}
}
