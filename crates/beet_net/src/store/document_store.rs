//! Documents through a [`BlobStore`], and the by-*location* half of schema
//! resolution.
//!
//! A data document may name its schema by pointing at another document
//! ([`FieldSchema::Document`]), resolved by default in the naming document's own
//! store, the `AncestorQuery<&BlobStore>` idiom. The index that answers such a
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
		path: &SmolPath,
	) -> Result<TypedDocument> {
		let bytes = self.get(path).await?;
		TypedDocument::read(
			resolver,
			path.as_str(),
			core::str::from_utf8(&bytes)?,
		)
		.await
	}

	/// Write `document` to `path`, as the byte-deterministic json a reopen
	/// re-saves identically.
	pub async fn insert_document(
		&self,
		path: &SmolPath,
		document: &TypedDocument,
	) -> Result {
		self.insert(path, document.to_json()?).await
	}
}

/// Marks a document whose located schema is being read, so a store churning
/// while the read is in flight does not issue a second one.
#[derive(Component)]
pub(crate) struct ReadingLocatedSchema;

/// Run condition for [`read_located_schemas`]: a document or a store arrived,
/// the two orders in which a located schema becomes readable.
pub(crate) fn located_schemas_may_be_readable(
	documents: Query<(), Added<DocumentSchema>>,
	stores: Query<(), Added<BlobStore>>,
) -> bool {
	!documents.is_empty() || !stores.is_empty()
}

/// Read the schema document each [`FieldSchema::Document`] names out of the
/// naming document's own store, into [`SchemaRegistry`]'s by-location index.
///
/// Lazy by decision, as `DocRef` resolution is: a document legitimately arrives
/// before the store that answers it, so finding no ancestor store is not an
/// error, it leaves the arm deferred until one arrives. Once the schema lands,
/// the read backstop tightens the invariant on the document that named it.
pub(crate) fn read_located_schemas(
	mut commands: Commands,
	async_commands: AsyncCommands,
	registry: Res<SchemaRegistry>,
	documents: Populated<
		(Entity, &DocumentSchema),
		Without<ReadingLocatedSchema>,
	>,
) {
	for (entity, schema) in documents.iter() {
		let FieldSchema::Document(path) = &schema.0 else {
			continue;
		};
		if registry.located(path).is_some() {
			continue;
		}
		// the resolver cannot hold a `Res` across an await, so the task carries
		// its own snapshot, which is what validates the arriving document.
		let (path, snapshot) = (path.clone(), registry.clone());
		commands.entity(entity).insert(ReadingLocatedSchema);
		async_commands.entity(entity).run(async move |entity| {
			let outcome = read_located_schema(&entity, snapshot, path).await;
			// always released, so a transient failure is retried when the next
			// document or store arrives rather than wedging this one
			entity
				.with(|mut entity| {
					entity.remove::<ReadingLocatedSchema>();
				})
				.await?;
			outcome
		});
	}
}

/// The read itself: the nearest ancestor store, the schema document it holds,
/// and its registration by location.
async fn read_located_schema(
	entity: &AsyncEntity,
	snapshot: SchemaRegistry,
	path: SmolPath,
) -> Result {
	let store = entity
		.with_state::<AncestorQuery<&BlobStore>, _>(|entity, query| {
			query.get(entity).cloned().ok()
		})
		.await?;
	let Some(store) = store else {
		return OK;
	};
	let schema = store
		.get_document(SchemaResolver::default().with_schemas(&snapshot), &path)
		.await?
		.to_schema()?;
	entity
		.world()
		.with(move |world: &mut World| {
			world
				.get_resource_or_init::<SchemaRegistry>()
				.insert_located(path, schema);
		})
		.await;
	OK
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// `{ label: String }`, the todo app's authored schema.
	fn todo_schema() -> ValueSchema {
		ValueSchema::Struct(StructSchema {
			name: Some("TodoItem".into()),
			allow_additional: false,
			fields: vec![NamedFieldSchema::new(
				"label",
				ValueSchema::String(default()),
			)],
		})
	}

	/// A store holding `schema.json` (the schema document) and `todos.json`
	/// (the data document naming it by location).
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
					FieldSchema::document("schema.json"),
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
			.get_document(resolver, &SmolPath::from("schema.json"))
			.await
			.unwrap()
			.to_schema()
			.unwrap()
			.xpect_eq(todo_schema());
	}

	/// Until the schema document is read the arm defers to a wildcard; after it
	/// is, the data document validates against the schema it named.
	#[beet_core::test]
	async fn a_located_schema_resolves_after_it_arrives() {
		let mut registry = SchemaRegistry::default();
		let store = todo_store().await;
		let path = SmolPath::from("schema.json");

		let mut document = store
			.get_document(
				SchemaResolver::default().with_schemas(&registry),
				&SmolPath::from("todos.json"),
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
	/// the registry without anything asking for it.
	#[beet_core::test]
	async fn the_store_walk_fills_the_located_index() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AsyncPlugin, StorePlugin));
		let store = todo_store().await;
		app.world_mut().spawn((store, children![DocumentSchema(
			FieldSchema::document("schema.json")
		)]));
		app.update_async().await;
		app.world()
			.resource::<SchemaRegistry>()
			.located(&SmolPath::from("schema.json"))
			.unwrap()
			.xpect_eq(todo_schema());
	}
}
