//! [`DocumentBlob`]: an editable document that lives in a store.
//!
//! Item 16's split, embodied. A `.bsx` file is the authored *original* state of
//! an application and no editor path ever writes one; everything the running
//! application may rewrite is a [`TypedDocument`] in a store. This is the
//! component that names one: it loads the `Document` / `DocumentSchema` pair the
//! binding layer already syncs onto its own entity, and writes the document back
//! whenever an edit changes it, so a typed character or a committed schema
//! survives the process.
//!
//! The file is reached through the [`Blob`] its derived [`BlobPath`] resolves
//! from the nearest ancestor store, so the read keys on `Changed<Blob>`: the
//! store arriving frames after the tree, the store above it being swapped, and
//! an external edit to the object all land the same way, a read that has not
//! happened yet rather than an error.
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::platform::sync::Arc;
use bevy::platform::sync::Mutex;

/// The [`TypedDocument`] at `path` in the nearest ancestor [`BlobStore`], loaded
/// onto this entity and written back on every edit.
///
/// Authoring a document is therefore naming its file:
///
/// ```rsx
/// <DocumentBlob path="todos.json">
///   <DynamicView/>
/// </DocumentBlob>
/// ```
///
/// A **schema** document (one whose declared schema is the meta-schema) also
/// registers into the [`SchemaRegistry`] by location as it lands, under the name
/// it declares for itself. That is what lets a data document composing only the
/// row schema (`List(Ref(Name("TodoItem")))`) resolve it without knowing where it
/// is stored, and it is the same [`SchemaRegistry::insert_located`] entry point
/// [`read_located_schemas`](super::read_located_schemas) uses, so the two ways a
/// schema document is reached can never disagree.
///
/// The document arrives frames after the tree that binds it is built, which is
/// deliberate and is what the widgets are built for: a binding into a document
/// that has not answered syncs nothing, and a form or view reading its schema
/// generates itself the moment it does. An edit made elsewhere (another
/// process, another tab on the same store) lands the same way, replacing the
/// document; the entity's own write-back is recognized by its stat and never
/// re-read.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_insert = hook_ext::component_hook(|doc: &DocumentBlob| BlobPath::derive(&doc.path)))]
pub struct DocumentBlob {
	/// The document's path within the nearest ancestor store.
	pub path: RelPath,
	/// Coalesces the reads: a burst of changes is at most one read in flight
	/// plus one queued behind it, and the queued one reads the latest bytes.
	#[reflect(ignore)]
	read_trigger: CoalescingTrigger,
	/// Coalesces the write-backs: a burst of edits is at most one write in
	/// flight plus one queued behind it, rather than a queue per keystroke, and
	/// the queued one re-reads the document so it persists the latest state.
	#[reflect(ignore)]
	write_trigger: CoalescingTrigger,
	/// The stat of the bytes this entity last wrote, so the change its own
	/// write-back raises is recognized and not read back.
	#[reflect(ignore)]
	last_written: LastWritten,
}

/// The [`BlobStat`] of the bytes a stored document's entity last wrote, shared
/// between its write task and its reads.
#[derive(Debug, Default, Clone)]
struct LastWritten(Arc<Mutex<Option<BlobStat>>>);

impl LastWritten {
	/// Whether `bytes` are the bytes this entity last wrote.
	fn matches(&self, bytes: &[u8]) -> bool {
		self.0
			.lock()
			.unwrap()
			.as_ref()
			.is_some_and(|written| written.matches(&BlobStat::of(bytes)))
	}

	fn set(&self, bytes: &[u8]) {
		*self.0.lock().unwrap() = Some(BlobStat::of(bytes));
	}
}

impl DocumentBlob {
	/// The document at `path` in the nearest ancestor store.
	pub fn new(path: impl Into<RelPath>) -> Self {
		Self {
			path: path.into(),
			..default()
		}
	}
}

/// Marks a [`DocumentBlob`] whose document has arrived, so the write-back only
/// ever mirrors an edit and never echoes the read that produced it.
#[derive(Component)]
pub struct DocumentBlobLoaded;

/// Read each [`DocumentBlob`] whose [`Blob`] changed out of its store onto its
/// own entity: the blob arriving, re-resolving, or the object changing.
pub(crate) fn read_document_blobs(
	mut async_commands: AsyncCommands,
	registry: Res<SchemaRegistry>,
	blobs: Populated<(Entity, &DocumentBlob, &Blob), Changed<Blob>>,
) {
	for (entity, doc, blob) in blobs.iter() {
		// the resolver cannot hold a `Res` across an await, so the task carries
		// its own snapshot, which is what validates the arriving document.
		let (snapshot, blob) = (registry.clone(), blob.clone());
		let (trigger, last_written) =
			(doc.read_trigger.clone(), doc.last_written.clone());
		async_commands
			.entity(entity)
			.queue_async(async move |entity| {
				trigger
					.run_flush(async || {
						read_document_blob(
							&entity,
							snapshot.clone(),
							&blob,
							&last_written,
						)
						.await
					})
					.await
			});
	}
}

/// The read itself: the object's bytes, the document they hold, and the pair
/// it lands as. Bytes this entity itself last wrote are its own write-back
/// surfacing through the store's watcher, and land nothing.
async fn read_document_blob(
	entity: &AsyncEntity,
	snapshot: SchemaRegistry,
	blob: &Blob,
	last_written: &LastWritten,
) -> Result {
	let bytes = blob.get().await?;
	if last_written.matches(&bytes) {
		return OK;
	}
	let path = blob.path().clone();
	let document = TypedDocument::read(
		SchemaResolver::default().with_schemas(&snapshot),
		path.as_str(),
		core::str::from_utf8(&bytes)?,
	)
	.await?;
	// a schema document is a `ValueSchema` stored as data, so its own declared
	// schema is the meta-schema by definition; that is the whole test for one.
	let schema = (document.schema == ValueSchema::type_ref::<ValueSchema>())
		.then(|| document.to_schema())
		.transpose()?;
	entity
		.with(move |mut entity| {
			// registered before the document lands, so the read backstop and
			// every binding see the schema in the frame the pair appears
			if let Some(schema) = schema {
				entity.world_scope(|world| {
					world
						.get_resource_or_init::<SchemaRegistry>()
						.insert_located(path, schema);
				});
			}
			// removed then re-added: a replace keeps the `added` tick, and the
			// write-back's echo guard reads `Added<DocumentBlobLoaded>`
			entity.remove::<DocumentBlobLoaded>();
			entity.insert((document.bundle(), DocumentBlobLoaded));
		})
		.await?;
	OK
}

/// Write each edited [`DocumentBlob`] back to its store.
pub(crate) fn write_document_blobs(
	mut async_commands: AsyncCommands,
	edited: Populated<
		(Entity, &DocumentBlob, &Blob),
		(With<DocumentBlobLoaded>, Changed<Document>),
	>,
	just_loaded: Query<(), Added<DocumentBlobLoaded>>,
) {
	for (entity, doc, blob) in edited.iter() {
		// the frame the read landed is not an edit: writing it back would echo
		// the file at itself and race a concurrent editor for nothing
		if just_loaded.contains(entity) {
			continue;
		}
		let (trigger, last_written, blob) = (
			doc.write_trigger.clone(),
			doc.last_written.clone(),
			blob.clone(),
		);
		async_commands
			.entity(entity)
			.queue_async(async move |entity| {
				trigger
					.run_flush(async || {
						write_document_blob(&entity, &blob, &last_written).await
					})
					.await
			});
	}
}

/// One write: the document as it stands *now*, so the retry queued behind an
/// in-flight write persists the latest edit rather than the one that queued it.
async fn write_document_blob(
	entity: &AsyncEntity,
	blob: &Blob,
	last_written: &LastWritten,
) -> Result {
	let subject = blob.path().clone();
	let document = entity
		.with(move |entity| -> Result<TypedDocument> {
			let value = entity
				.get::<Document>()
				.ok_or_else(|| {
					bevyhow!("`{subject}` holds no document to write")
				})?
				.0
				.clone();
			let schema = entity
				.get::<DocumentSchema>()
				.ok_or_else(|| {
					bevyhow!("`{subject}` declares no schema to write")
				})?
				.0
				.clone();
			TypedDocument::new(schema, value).xok()
		})
		.await??;
	// the byte-deterministic json a reopen re-saves identically
	let json = document.to_json()?;
	blob.insert(json.clone()).await?;
	last_written.set(json.as_bytes());
	OK
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// `{ label: String, done: bool }` named for the registry, the todo app's
	/// row schema.
	fn todo_schema() -> ValueSchema {
		ValueSchema::Struct(StructSchema {
			name: Some("TodoItem".into()),
			description: None,
			allow_additional: false,
			fields: vec![
				NamedFieldSchema::new("label", ValueSchema::String(default())),
				NamedFieldSchema::new("done", ValueSchema::Bool(default())),
			],
		})
	}

	/// The list of rows the data document composes by reference, the entry's own
	/// shape: the schema document holds the row, the data document the list.
	fn rows_schema() -> ValueSchema {
		ValueSchema::List(ListSchema {
			item: Box::new(ValueSchema::reference("TodoItem")),
			min_items: None,
			max_items: None,
			unique: false,
		})
	}

	/// A store holding the app's two documents, as the entry ships them. The
	/// concrete [`InMemoryStore`] so spawning it subscribes its watcher, the way
	/// a store declared in markup does.
	async fn todo_store() -> InMemoryStore {
		let inner = InMemoryStore::new();
		let store = BlobStore::new(inner.clone());
		store
			.insert_document(
				&RelPath::from("schema.json"),
				&TypedDocument::schema_document(&todo_schema()).unwrap(),
			)
			.await
			.unwrap();
		store
			.insert_document(
				&RelPath::from("todos.json"),
				&TypedDocument::new(
					rows_schema(),
					value!([{ "label": "buy milk", "done": false }]),
				),
			)
			.await
			.unwrap();
		inner
	}

	/// An app under a store: the schema document and the data document, spawned
	/// as the entry authors them.
	async fn todo_app() -> (App, Entity, Entity) {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AsyncPlugin, StorePlugin));
		let store = todo_store().await;
		let schema =
			app.world_mut().spawn(DocumentBlob::new("schema.json")).id();
		let data = app.world_mut().spawn(DocumentBlob::new("todos.json")).id();
		app.world_mut().spawn(store).add_children(&[schema, data]);
		app.update_async().await;
		(app, schema, data)
	}

	/// The app's store, for a test writing beside the entity.
	fn store_of(app: &mut App) -> BlobStore {
		app.world_mut()
			.query_once::<&BlobStore>()
			.into_iter()
			.next()
			.cloned()
			.unwrap()
	}

	/// Counts every document landing on an entity after installation.
	#[derive(Default, Resource)]
	struct Loads(usize);

	fn count_loads(app: &mut App) {
		app.init_resource::<Loads>().add_systems(
			Update,
			|loaded: Query<(), Added<DocumentBlobLoaded>>,
			 mut loads: ResMut<Loads>| {
				loads.0 += loaded.iter().count();
			},
		);
	}

	/// Both documents land as the pair every binding resolves against, and the
	/// schema document joins the one namespace under the name it declares, so
	/// the data document's `List(Ref("TodoItem"))` resolves.
	#[beet_core::test]
	async fn a_stored_document_lands_as_a_document() {
		let (app, schema, data) = todo_app().await;
		app.world()
			.get::<Document>(data)
			.unwrap()
			.0
			.clone()
			.xpect_eq(value!([{ "label": "buy milk", "done": false }]));
		app.world()
			.get::<DocumentSchema>(data)
			.unwrap()
			.0
			.clone()
			.xpect_eq(rows_schema());
		app.world()
			.get::<Document>(schema)
			.unwrap()
			.0
			.clone()
			.into_serde::<ValueSchema>()
			.unwrap()
			.xpect_eq(todo_schema());
		app.world()
			.resource::<SchemaRegistry>()
			.get("TodoItem")
			.unwrap()
			.clone()
			.xpect_eq(todo_schema());
	}

	/// An edit to the loaded document reaches the store, which is what makes a
	/// typed character or a committed schema outlive the process. The read
	/// itself is not an edit, so nothing is written until one happens.
	#[beet_core::test]
	async fn an_edit_reaches_the_store() {
		let (mut app, _, data) = todo_app().await;
		let store = store_of(&mut app);

		app.world_mut().get_mut::<Document>(data).unwrap().0 = value!([
			{ "label": "buy milk", "done": true },
			{ "label": "walk dog", "done": false },
		]);
		app.update_async().await;

		store
			.get_document(
				SchemaResolver::default(),
				&RelPath::from("todos.json"),
			)
			.await
			.unwrap()
			.value
			.xpect_eq(value!([
				{ "label": "buy milk", "done": true },
				{ "label": "walk dog", "done": false },
			]));
	}

	/// An edit made through another handle on the same store (another process,
	/// another tab) lands on the entity: the object's [`Blob`] is marked
	/// changed and the document is read again.
	#[beet_core::test]
	async fn an_external_write_is_read() {
		let (mut app, _, data) = todo_app().await;
		count_loads(&mut app);
		store_of(&mut app)
			.insert_document(
				&RelPath::from("todos.json"),
				&TypedDocument::new(rows_schema(), value!([])),
			)
			.await
			.unwrap();
		app.update_async().await;
		app.world().resource::<Loads>().0.xpect_eq(1);
		app.world()
			.get::<Document>(data)
			.unwrap()
			.0
			.clone()
			.xpect_eq(value!([]));
	}

	/// The entity's own write-back surfaces through the store's watcher like
	/// any other change, and is recognized by its bytes: nothing is read back,
	/// so a write never clobbers the edit that follows it.
	#[beet_core::test]
	async fn a_write_back_is_not_read() {
		let (mut app, _, data) = todo_app().await;
		count_loads(&mut app);
		app.world_mut().get_mut::<Document>(data).unwrap().0 =
			value!([{ "label": "buy milk", "done": true }]);
		app.update_async().await;
		app.world().resource::<Loads>().0.xpect_eq(0);
	}
}
