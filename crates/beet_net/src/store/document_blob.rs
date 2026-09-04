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
//! Both halves are lazy in the shape [`DocRef`] resolution settled on: a store
//! that has not arrived is not an error, only a read that has not happened yet.
use crate::prelude::*;
use beet_core::prelude::*;

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
/// generates itself the moment it does.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct DocumentBlob {
	/// The document's path within the nearest ancestor store.
	pub path: SmolPath,
	/// Coalesces the write-backs: a burst of edits is at most one write in
	/// flight plus one queued behind it, rather than a queue per keystroke, and
	/// the queued one re-reads the document so it persists the latest state.
	#[reflect(ignore)]
	trigger: CoalescingTrigger,
}

impl DocumentBlob {
	/// The document at `path` in the nearest ancestor store.
	pub fn new(path: impl Into<SmolPath>) -> Self {
		Self {
			path: path.into(),
			trigger: default(),
		}
	}
}

/// Marks a [`DocumentBlob`] whose read is in flight, so a store churning while
/// it runs does not issue a second one.
#[derive(Component)]
pub(crate) struct ReadingDocumentBlob;

/// Marks a [`DocumentBlob`] whose document has arrived, so the write-back only
/// ever mirrors an edit and never echoes the read that produced it.
#[derive(Component)]
pub struct DocumentBlobLoaded;

/// Run condition for [`read_document_blobs`]: a blob or a store arrived, the two
/// orders in which a stored document becomes readable.
pub(crate) fn document_blobs_may_be_readable(
	blobs: Query<(), Added<DocumentBlob>>,
	stores: Query<(), Added<BlobStore>>,
) -> bool {
	!blobs.is_empty() || !stores.is_empty()
}

/// Read each [`DocumentBlob`] out of its nearest ancestor store onto its own
/// entity.
pub(crate) fn read_document_blobs(
	mut commands: Commands,
	async_commands: AsyncCommands,
	registry: Res<SchemaRegistry>,
	blobs: Populated<
		(Entity, &DocumentBlob),
		(Without<ReadingDocumentBlob>, Without<DocumentBlobLoaded>),
	>,
) {
	for (entity, blob) in blobs.iter() {
		// the resolver cannot hold a `Res` across an await, so the task carries
		// its own snapshot, which is what validates the arriving document.
		let (path, snapshot) = (blob.path.clone(), registry.clone());
		commands.entity(entity).insert(ReadingDocumentBlob);
		async_commands.entity(entity).run(async move |entity| {
			let outcome = read_document_blob(&entity, snapshot, path).await;
			// always released, so a transient failure is retried when the next
			// blob or store arrives rather than wedging this one
			entity
				.with(|mut entity| {
					entity.remove::<ReadingDocumentBlob>();
				})
				.await?;
			outcome
		});
	}
}

/// The read itself: the nearest ancestor store, the document it holds, and the
/// pair it lands as.
async fn read_document_blob(
	entity: &AsyncEntity,
	snapshot: SchemaRegistry,
	path: SmolPath,
) -> Result {
	let Some(store) = ancestor_store(entity).await? else {
		return OK;
	};
	let document = store
		.get_document(SchemaResolver::default().with_schemas(&snapshot), &path)
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
			entity.insert((document.bundle(), DocumentBlobLoaded));
		})
		.await?;
	OK
}

/// Write each edited [`DocumentBlob`] back to its store.
pub(crate) fn write_document_blobs(
	async_commands: AsyncCommands,
	edited: Populated<
		(Entity, &DocumentBlob),
		(With<DocumentBlobLoaded>, Changed<Document>),
	>,
	just_loaded: Query<(), Added<DocumentBlobLoaded>>,
) {
	for (entity, blob) in edited.iter() {
		// the frame the read landed is not an edit: writing it back would echo
		// the file at itself and race a concurrent editor for nothing
		if just_loaded.contains(entity) {
			continue;
		}
		let (path, trigger) = (blob.path.clone(), blob.trigger.clone());
		async_commands.entity(entity).run(async move |entity| {
			trigger
				.run_flush(async move || {
					write_document_blob(&entity, path.clone()).await
				})
				.await
		});
	}
}

/// One write: the document as it stands *now*, so the retry queued behind an
/// in-flight write persists the latest edit rather than the one that queued it.
async fn write_document_blob(entity: &AsyncEntity, path: SmolPath) -> Result {
	let Some(store) = ancestor_store(entity).await? else {
		return OK;
	};
	let subject = path.clone();
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
	store.insert_document(&path, &document).await
}

/// The nearest ancestor [`BlobStore`], or `None` while none has arrived.
async fn ancestor_store(entity: &AsyncEntity) -> Result<Option<BlobStore>> {
	entity
		.with_state::<AncestorQuery<&BlobStore>, _>(|entity, query| {
			query.get(entity).cloned().ok()
		})
		.await
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

	/// A store holding the app's two documents, as the entry ships them.
	async fn todo_store() -> BlobStore {
		let store = BlobStore::temp();
		store
			.insert_document(
				&SmolPath::from("schema.json"),
				&TypedDocument::schema_document(&todo_schema()).unwrap(),
			)
			.await
			.unwrap();
		store
			.insert_document(
				&SmolPath::from("todos.json"),
				&TypedDocument::new(
					rows_schema(),
					value!([{ "label": "buy milk", "done": false }]),
				),
			)
			.await
			.unwrap();
		store
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
		let root = app
			.world_mut()
			.spawn((store, children![]))
			.add_children(&[schema, data])
			.id();
		let _ = root;
		app.update_async().await;
		(app, schema, data)
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
		let store = app
			.world_mut()
			.query_once::<&BlobStore>()
			.into_iter()
			.next()
			.cloned()
			.unwrap();

		app.world_mut().get_mut::<Document>(data).unwrap().0 = value!([
			{ "label": "buy milk", "done": true },
			{ "label": "walk dog", "done": false },
		]);
		app.update_async().await;

		store
			.get_document(
				SchemaResolver::default(),
				&SmolPath::from("todos.json"),
			)
			.await
			.unwrap()
			.value
			.xpect_eq(value!([
				{ "label": "buy milk", "done": true },
				{ "label": "walk dog", "done": false },
			]));
	}
}
