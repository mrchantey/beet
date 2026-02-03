use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::query::QueryEntityError;
use bevy::ecs::query::ROQueryItem;

/// In-memory representation of a document.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Component)]
pub struct Document(serde_json::Value);

/// Marker component for the global application-level document.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Component)]
pub struct AppDocument;


/// A reference to a specific field For tools that interact with documents,
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Reflect,
	Component,
)]
#[reflect(Component)]
pub struct FieldRef {
	/// The path to the document
	pub document: DocumentPath,
	/// The path to the field within the document
	pub field_path: Vec<FieldPath>,
	/// By default fields are initialized with a default value
	/// if missing, otherwise an error is triggered.
	pub error_on_missing: bool,
}




/// Specify the document to operate on, either the global application-level document,
/// or another by id.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
pub enum DocumentPath {
	/// The document for this card.
	#[default]
	Card,
	/// The global application-level document
	App,
	/// A specific document by entity id
	Entity(Entity),
}



/// A path to a specific field within a document
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
pub enum FieldPath {
	/// Access an array element by index
	ArrayIndex(usize),
	/// Access an object field by key
	ObjectKey(String),
}


/// System parameter for ergonomic interaction with documents.
#[derive(SystemParam)]
pub struct DocumentQuery<'w, 's> {
	ancestors: Query<'w, 's, &'static ChildOf>,
	doc_query: Query<'w, 's, &'static Document>,
	app_doc_query: Query<'w, 's, Entity, With<AppDocument>>,
	card_query: Query<'w, 's, &'static Card>,
	commands: Commands<'w, 's>,
}

impl<'w, 's> DocumentQuery<'w, 's> {
	/// Get the entity for the given [`DocumentPath`],
	/// which may or may not contain an initialized document.
	pub fn document_entity(
		&mut self,
		entity: Entity,
		path: &DocumentPath,
	) -> Result<Entity> {
		match path {
			DocumentPath::Card => self
				.ancestors
				.iter_ancestors(entity)
				.find(|entity| self.card_query.get(*entity).is_ok())
				.ok_or_else(|| {
					bevyhow!("no card ancestor found for entity {}", entity)
				})?,
			DocumentPath::App => self
				.app_doc_query
				.iter()
				.next()
				.unwrap_or_else(|| self.commands.spawn(AppDocument).id()),
			DocumentPath::Entity(entity) => *entity,
		}
		.xok()
	}

	/// Returns the query item for the agent of the given action.
	pub fn get(
		&mut self,
		entity: Entity,
		path: &DocumentPath,
	) -> Result<ROQueryItem<'_, 's, &Document>> {
		let agent = self.document_entity(entity, path)?;
		self.doc_query.get(agent)?.xok()
	}


	/// Returns the mutable query item for the agent of the given action.
	/// Returns the mutable query item for the agent of the given action.
	pub fn get_mut(
		&mut self,
		entity: Entity,
		path: &DocumentPath,
	) -> Result<ROQueryItem<'_, 's, &mut Document>> {
		let agent = self.document_entity(entity, path)?;
		self.doc_query.get_mut(agent)?.xok()
	}

	pub async fn with_async<O>(
		entity: AsyncEntity,
		document: &DocumentPath,
		func: impl FnOnce(&mut Document) -> O,
	) -> Result<O> {
		let id = entity.id();
		entity.world().with_then(|world| {
			world.run_system_cached_with(
				|In((entity, doc_path)): In<(Entity, DocumentPath)>,
					mut doc_query: DocumentQuery| {
					let doc_mut = doc_query.get_mut(entity, &doc_path)?;
					func(doc_mut).xok()
				},
				(id, document.clone()),
			)
		})
	}
}
