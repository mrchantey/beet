use super::*;
use crate::prelude::*;

/// Plugin that enables document synchronization for field values.
///
/// This plugin:
/// - Links [`FieldRef`] components to their associated documents
/// - Automatically updates [`Value`] when documents change (read path)
/// - Writes a changed [`Value`] back into its document field (write-back)
///
/// # Example
///
/// ```
/// use beet_core::prelude::*;
///
/// let mut world = DocumentPlugin::world();
///
/// // Create a document with a value child
/// world.spawn((
///     Document::new(val!({ "name": "Alice" })),
///     children![(Value::default(), FieldRef::new("name"))],
/// ));
///
/// // After update, Value will contain Str("Alice")
/// world.update_local();
///
/// let values = world.query_once::<(&Value, &FieldRef)>();
/// let (value, _) = &values[0];
/// assert_eq!(*value, &Value::Str("Alice".into()));
/// ```
#[derive(Default)]
pub struct DocumentPlugin;

impl Plugin for DocumentPlugin {
	fn build(&self, app: &mut App) {
		app
			// Register document types
			.register_type::<Document>()
			.register_type::<PropsDocument>()
			.register_type::<DocumentSchema>()
			.register_type::<DocumentPath>()
			.register_type::<OnMissingField>()
			.register_type::<FieldRef>()
			.register_type::<SourceFieldRef>()
			.register_type::<LayoutContent>()
			.register_type::<LayoutContentOf>()
			.register_type::<BindingTarget>()
			.register_type::<FieldSegment>()
			.register_type::<DocumentScope>()
			.register_type::<ResolvedFieldPath>()
			.register_type::<Value>()
			.register_type::<ValueSchema>();

		app
			// Add observers and systems
			.add_observer(link_field_to_document)
			.add_observer(unlink_field_from_document)
			.add_observer(resolve_field_path);

		#[cfg(feature = "json")]
		app.register_type::<ReflectFieldRef>()
			.register_type::<ResourceFieldRef>();

		// the binding syncs mirror a `Value` to/from a reflected component or
		// resource field, the generalization of the `Value`-component bind. They
		// run after the document drives `Value` so the read direction lands first.
		// Resources sync before components so a co-located pair (eg
		// `<MyComponent value=@res:Type.field>`) seeds outside-in: the resource
		// fills the `Value`, which the component sync then propagates.
		#[cfg(feature = "json")]
		let reflect_sync =
			(sync_resource_field_bindings, sync_reflect_field_bindings).chain();
		#[cfg(not(feature = "json"))]
		let reflect_sync = || {};

		// the chain lives in its own on-demand schedule so a one-shot render can
		// run it to settlement ([`DocumentSync::settle`]) without driving the
		// whole main loop; the realtime path drives it from `PreUpdate`.
		app.init_schedule(DocumentSync);
		app.add_systems(
			DocumentSync,
			(
				update_resolved_field_paths.run_if(resolved_paths_need_update),
				sync_schema.run_if(schema_needs_sync),
				sync_document_to_local,
				sync_resolved_path_changes,
				// after the read path so a same-pass conflict resolves source-wins,
				// before the write-back so the mirrored value lands the same pass.
				sync_source_field_refs,
				// field bindings run between the read path and the write-back, so a
				// document change reaches the component/resource and an edit there
				// reaches the document, both within one pass.
				reflect_sync,
				sync_local_to_document,
				update_reactive_children,
			)
				.chain(),
		);
		// with `bevy_async` the per-frame run waits for the async sync point so an
		// async field write (eg refresh_blob_store_list) lands the same pass.
		#[cfg(feature = "bevy_async")]
		app.add_systems(
			PreUpdate,
			run_document_sync
				.after(async_world_sync_point::<BeetAsyncSyncPoint>),
		);
		#[cfg(not(feature = "bevy_async"))]
		app.add_systems(PreUpdate, run_document_sync);
	}
}

/// Per-frame driver: one [`DocumentSync`] pass.
fn run_document_sync(world: &mut World) { world.run_schedule(DocumentSync); }

/// The document sync chain's schedule: one read/write pass between every
/// [`Document`], its bound [`Value`] entities, and the reflected
/// component/resource bindings.
///
/// Driven from `PreUpdate` each frame, and on demand by render paths via
/// [`DocumentSync::settle`].
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct DocumentSync;

impl DocumentSync {
	/// One sync pass may not settle a multi-hop binding chain (eg resource ->
	/// `Value` <-> props field -> body binding): each hop lands one pass later.
	/// A well-formed tree settles within a few passes; the cap only guards a
	/// pathological cycle (the inequality guards make those converge too).
	const MAX_SETTLE_PASSES: usize = 8;

	/// Run the sync chain until a pass changes no [`Value`] or [`Document`],
	/// so a one-shot render (eg HTML SSR) observes fully synced bindings.
	///
	/// A no-op when the schedule is not registered (no [`DocumentPlugin`]).
	pub fn settle(world: &mut World) {
		for _ in 0..Self::MAX_SETTLE_PASSES {
			let before = world.change_tick();
			if world.try_run_schedule(DocumentSync).is_err() {
				return;
			}
			let this_run = world.change_tick();
			let changed =
				world.query::<Ref<Value>>().iter(world).any(|value| {
					value.last_changed().is_newer_than(before, this_run)
				}) || world.query::<Ref<Document>>().iter(world).any(
					|document| {
						document.last_changed().is_newer_than(before, this_run)
					},
				);
			if !changed {
				return;
			}
		}
		warn!(
			"document sync did not settle within {} passes",
			Self::MAX_SETTLE_PASSES
		);
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	fn text_field_syncs_on_insert() {
		let mut world = DocumentPlugin::world();

		// Create document with a value child
		world.spawn((Document::new(val!({ "greeting": "Hello" })), children![
			(Value::default(), FieldRef::new("greeting"))
		]));

		// Run update to trigger sync
		world.update_local();

		// Verify value was updated
		let (value, _) = &world.query_once::<(&Value, &FieldRef)>()[0];
		(*value).clone().xpect_eq(Value::Str("Hello".into()));
	}

	#[beet_core::test]
	fn text_field_syncs_on_document_change() {
		let mut world = DocumentPlugin::world();

		// Create document with a value child
		let card = world
			.spawn((Document::new(val!({ "count": 0i64 })), children![(
				Value::default(),
				FieldRef::new("count")
			)]))
			.id();

		world.update_local();

		// Initial value
		let (value, _) = &world.query_once::<(&Value, &FieldRef)>()[0];
		(*value).clone().xpect_eq(Value::Int(0));

		// Update the document
		world.entity_mut(card).get_mut::<Document>().unwrap().0 =
			val!({ "count": 42i64 });

		world.update_local();

		// Value should be updated
		let (value, _) = &world.query_once::<(&Value, &FieldRef)>()[0];
		(*value).clone().xpect_eq(Value::Int(42));
	}

	#[beet_core::test]
	fn text_field_with_nested_path() {
		let mut world = DocumentPlugin::world();

		world.spawn((
			Document::new(val!({ "user": { "name": "Bob" } })),
			children![(Value::default(), FieldRef::new(vec!["user", "name"]))],
		));

		world.update_local();

		let (value, _) = &world.query_once::<(&Value, &FieldRef)>()[0];
		(*value).clone().xpect_eq(Value::Str("Bob".into()));
	}

	#[beet_core::test]
	fn multiple_text_fields_same_document() {
		let mut world = DocumentPlugin::world();

		world.spawn((
			Document::new(val!({
				"first": "Alice",
				"second": "Bob"
			})),
			children![
				(Value::default(), FieldRef::new("first")),
				(Value::default(), FieldRef::new("second"))
			],
		));

		world.update_local();

		let results: Vec<_> = world
			.query_once::<(&Value, &FieldRef)>()
			.iter()
			.map(|(value, _)| (*value).clone())
			.collect();

		results.contains(&Value::Str("Alice".into())).xpect_true();
		results.contains(&Value::Str("Bob".into())).xpect_true();
	}

	#[beet_core::test]
	fn text_block_with_field_ref() {
		let mut world = DocumentPlugin::world();

		world.spawn((Document::new(val!({ "name": "World" })), children![
			Value::Str("Hello, ".into()),
			(FieldRef::new("name"), Value::default()),
		]));

		world.update_local();

		// Find the value content with field ref
		let results: Vec<_> = world
			.query_once::<(&Value, Option<&FieldRef>)>()
			.iter()
			.map(|(value, field)| ((*value).clone(), field.is_some()))
			.collect();

		// One should be static "Hello, ", one should be dynamic "World"
		results
			.iter()
			.any(|(value, has_field)| {
				*value == Value::Str("Hello, ".into()) && !has_field
			})
			.xpect_true();
		results
			.iter()
			.any(|(value, has_field)| {
				*value == Value::Str("World".into()) && *has_field
			})
			.xpect_true();
	}
}
