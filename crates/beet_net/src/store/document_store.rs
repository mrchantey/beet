//! Documents through a [`BlobStore`], and the by-*location* half of schema
//! resolution.
//!
//! A state document may name its schema by pointing at another document
//! ([`SchemaRef::Document`]), resolved in the naming document's own package
//! ([`PackageFilesQuery`]). The index that answers such a
//! reference is [`SchemaRegistry`]'s by-location one, which sits beside the
//! by-name namespace rather than inside it; a store sits above the schema layer
//! in the crate graph, so the reader that fills that index lives here.
use crate::prelude::*;
use beet_core::prelude::*;

impl BlobStore {
	/// Read the [`TypedDocument`] at `path`, validating it against its own
	/// schema exactly as [`TypedDocument::read`] does and naming `path` on
	/// failure.
	pub async fn get_document(
		&self,
		resolver: SchemaResolver<'_>,
		path: &RelPath,
	) -> Result<TypedDocument> {
		let bytes =
			MediaBytes::new(document_media_type(path), self.get(path).await?);
		TypedDocument::read(resolver, path.as_str(), &bytes).await
	}

	/// Write `document` to `path`, in the byte-deterministic form a reopen
	/// re-saves identically.
	pub async fn insert_document(
		&self,
		path: &RelPath,
		document: &TypedDocument,
	) -> Result {
		let (_, bytes) = document.to_bytes(document_media_type(path))?.take();
		self.insert(path, bytes).await
	}
}

/// The format a document at `path` is stored in: the one its extension names,
/// json for a path naming none.
pub(crate) fn document_media_type(path: &RelPath) -> MediaType {
	path.media_type().unwrap_or(MediaType::Json)
}

/// Run condition for [`read_located_schemas`]: a document or a store arrived,
/// the two orders in which a located schema becomes readable.
pub(crate) fn located_schemas_may_be_readable(
	documents: Query<(), Added<DocumentSchema>>,
	stores: Query<(), Added<BlobStore>>,
) -> bool {
	!documents.is_empty() || !stores.is_empty()
}

/// Read the schema document each [`SchemaRef::Document`] names out of the
/// naming document's own package, into [`SchemaRegistry`]'s by-location index.
///
/// Lazy by decision, as `DocRef` resolution is: a document legitimately arrives
/// before the store that answers it, so finding no ancestor store is not an
/// error, it leaves the arm deferred until one arrives. Once the schema lands,
/// the read backstop tightens the invariant on the document that named it.
pub(crate) fn read_located_schemas(
	mut commands: Commands,
	mut async_commands: AsyncCommands,
	registry: Res<SchemaRegistry>,
	documents: Populated<(Entity, &DocumentSchema, Option<&Loading>)>,
) {
	for (entity, schema, loading) in documents.iter() {
		let ValueSchema::Ref(SchemaRef::Document(path)) = &schema.0 else {
			continue;
		};
		let document = path.clone();
		if registry.located(&document.rel_path()).is_some()
			|| !Loading::may_read(loading)
		{
			continue;
		}
		// the resolver cannot hold a `Res` across an await, so the task carries
		// its own snapshot, which is what validates the arriving document.
		let snapshot = registry.clone();
		commands.entity(entity).insert(Loading::InFlight);
		async_commands
			.entity(entity)
			.queue_async(async move |entity| {
				let outcome =
					read_located_schema(&entity, snapshot, document).await;
				// settled whatever the outcome, so a transient failure is retried
				// when the next document or store arrives rather than wedging
				// this one
				Loading::settle(&entity, &outcome).await?;
				outcome.map(|_| ())
			});
	}
}

/// The read itself: the naming document's package, the schema document it holds,
/// and its registration by location.
async fn read_located_schema(
	entity: &AsyncEntity,
	snapshot: SchemaRegistry,
	document: DocPath,
) -> Result<Read> {
	let path = document.rel_path();
	// `None` while no store has arrived above it: one legitimately lands frames
	// after the tree that reads it
	let Some(blob) = entity
		.with_state::<PackageFilesQuery, _>(move |entity, packages| {
			packages.resolve(entity, &document).ok()
		})
		.await?
	else {
		return Ok(Read::NoStore);
	};
	let blob = blob.complete().await?;
	let full = blob.path().clone();
	let schema = blob
		.store()
		.get_document(SchemaResolver::default().with_schemas(&snapshot), &full)
		.await?
		.to_schema()?
		.resolve_documents(&full);
	entity
		.world()
		.with(move |world: &mut World| {
			let mut registry = world.get_resource_or_init::<SchemaRegistry>();
			registry.alias_located(path, full.clone());
			registry.insert_located(full, schema);
		})
		.await;
	Ok(Read::Landed)
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// `{ label: String }`, the todo app's authored schema.
	fn todo_schema() -> ValueSchema {
		ValueSchema::Struct(StructSchema {
			name: Some("TodoItem".into()),
			description: None,
			allow_additional: false,
			fields: vec![NamedFieldSchema::new(
				"label",
				ValueSchema::String(default()),
			)],
		})
	}

	/// A store holding `schema.json` (the schema document) and `todos.json`
	/// (the state document naming it by location).
	async fn todo_store() -> BlobStore {
		let store = BlobStore::temp();
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
					ValueSchema::document("schema.json"),
					value!({ "label": "buy milk" }),
				),
			)
			.await
			.unwrap();
		store
	}

	#[beet_core::test]
	async fn a_document_round_trips_through_a_store() {
		let registry = SchemaRegistry::default();
		let resolver = SchemaResolver::default().with_schemas(&registry);
		let store = todo_store().await;
		store
			.get_document(resolver, &RelPath::from("schema.json"))
			.await
			.unwrap()
			.to_schema()
			.unwrap()
			.xpect_eq(todo_schema());
	}

	/// Until the schema document is read the arm defers to a wildcard; after it
	/// is, the state document validates against the schema it named.
	#[beet_core::test]
	async fn a_located_schema_resolves_after_it_arrives() {
		let mut registry = SchemaRegistry::default();
		let store = todo_store().await;
		let path = RelPath::from("schema.json");

		let mut document = store
			.get_document(
				SchemaResolver::default().with_schemas(&registry),
				&RelPath::from("todos.json"),
			)
			.await
			.unwrap();
		document
			.schema
			.resolve(SchemaResolver::default().with_schemas(&registry))
			.unwrap()
			.xpect_eq(ValueSchema::Any);

		let schema = store
			.get_document(
				SchemaResolver::default().with_schemas(&registry),
				&path,
			)
			.await
			.unwrap()
			.to_schema()
			.unwrap();
		registry.insert_located(path, schema);

		let resolver = SchemaResolver::default().with_schemas(&registry);
		document
			.schema
			.resolve(resolver)
			.unwrap()
			.xpect_eq(todo_schema());
		document.assert_valid(resolver, "todos.json").await.unwrap();
		// and the backstop now bites on data the arrived schema rejects
		document.value = value!({});
		document
			.assert_valid(resolver, "todos.json")
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("label");
	}

	/// The ECS half: a document under a store has its located schema read into
	/// the registry without anything asking for it, by whichever of its names
	/// the reference wrote.
	#[beet_core::test]
	async fn the_store_walk_fills_the_located_index() {
		for authored in ["schema.json", "schema"] {
			let mut app = App::new();
			app.add_plugins((MinimalPlugins, AsyncPlugin, StorePlugin));
			let store = todo_store().await;
			app.world_mut().spawn((store, children![DocumentSchema(
				ValueSchema::document(authored)
			)]));
			app.update_async().await;
			let registry = app.world().resource::<SchemaRegistry>();
			for path in [authored, "schema.json"] {
				registry
					.located(&RelPath::from(path))
					.unwrap()
					.xpect_eq(todo_schema());
			}
		}
	}
}
