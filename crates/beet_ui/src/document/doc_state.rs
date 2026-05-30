use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;
use core::marker::PhantomData;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;

/// A typed, keyed handle to a document field, Beet's atom.
///
/// Sugar over [`FieldRef`]: the same change-detection sync, with a typed default
/// and ergonomic get/set via [`DocStateQuery`]. A state lives in a single field
/// of its document, resolved by [`DocumentPath`] (default
/// [`DocumentPath::Ancestor`]).
///
/// ## Placement is explicit
///
/// Widgets never auto-scope a [`Document`]. The author decides where state lives:
/// - explicit keys ([`DocState::new`]) are shared, singleton state, one value per
///   document, ie a Recoil-style atom.
/// - per-instance state uses [`DocState::inline`] (a process-unique key) **or** a
///   dedicated ancestor [`Document`] per instance.
///
/// Two states with the *same* explicit key in the *same* document collide; that
/// is an author error to resolve with a distinct key,
/// [`inline`](DocState::inline), or a separate document.
pub struct DocState<T> {
	/// The document this state resolves against.
	document: DocumentPath,
	/// The key (field path) within the document.
	key: FieldPath,
	/// The default value, pre-serialized so `DocState<T>` is `Clone` regardless
	/// of whether `T` is.
	default: Value,
	_marker: PhantomData<fn() -> T>,
}

// Hand-written so `T` is never constrained, the `PhantomData<fn() -> T>` means
// the stored data is always `Clone` + `Debug` regardless of `T`.
impl<T> Clone for DocState<T> {
	fn clone(&self) -> Self {
		Self {
			document: self.document.clone(),
			key: self.key.clone(),
			default: self.default.clone(),
			_marker: PhantomData,
		}
	}
}

impl<T> core::fmt::Debug for DocState<T> {
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		f.debug_struct("DocState")
			.field("document", &self.document)
			.field("key", &self.key)
			.field("default", &self.default)
			.finish()
	}
}

impl<T> DocState<T>
where
	T: Default + Serialize + DeserializeOwned + Typed,
{
	/// Declare a state with an explicit key, ie a shared singleton atom.
	pub fn new<M>(key: impl IntoFieldPath<M>) -> Self {
		Self {
			document: DocumentPath::default(),
			key: key.into_field_path(),
			default: Value::from_serde(&T::default()).unwrap_or_default(),
			_marker: PhantomData,
		}
	}

	/// Declare a state with a process-unique `"inline_{N}"` key, ie per-instance
	/// widget state that never collides with another instance.
	pub fn inline() -> Self {
		/// Process-global counter backing [`DocState::inline`] keys.
		static INLINE_KEY_COUNTER: AtomicUsize = AtomicUsize::new(0);
		let index = INLINE_KEY_COUNTER.fetch_add(1, Ordering::Relaxed);
		Self::new(format!("inline_{index}"))
	}

	/// Override the default value used when the field is missing.
	pub fn with_default(mut self, value: T) -> Self {
		self.default = Value::from_serde(&value).unwrap_or_default();
		self
	}

	/// Override the [`DocumentPath`] this state resolves against.
	pub fn with_document_path(mut self, path: DocumentPath) -> Self {
		self.document = path;
		self
	}

	/// Lower to the [`FieldRef`], carrying the typed default as
	/// [`OnMissingField::Init`] so a missing field is initialized on first touch.
	pub fn field(&self) -> FieldRef {
		FieldRef {
			document: self.document.clone(),
			field_path: self.key.clone(),
			on_missing: OnMissingField::Init {
				value: self.default.clone(),
			},
		}
	}

	/// Read the current value against `entity`, initializing the field with its
	/// default if missing.
	pub fn get(&self, entity: &mut EntityWorldMut) -> Result<T> {
		entity.with_state::<DocStateQuery, _>(|subject, mut states| {
			states.get(subject, self)
		})
	}

	/// Write a new value against `entity`, type-checked against the schema.
	pub fn set(&self, entity: &mut EntityWorldMut, value: T) -> Result {
		entity.with_state::<DocStateQuery, _>(|subject, mut states| {
			states.set(subject, self, value)
		})
	}

	/// Read, mutate, and write back in one step against `entity`.
	pub fn update(
		&self,
		entity: &mut EntityWorldMut,
		func: impl FnOnce(&mut T),
	) -> Result {
		entity.with_state::<DocStateQuery, _>(|subject, mut states| {
			states.update(subject, self, func)
		})
	}
}

/// Marker disambiguating the [`DocState`] markup-read [`IntoScene`] impl.
#[cfg(feature = "scene")]
pub struct SceneDocStateMarker;

/// Read a [`DocState`] in markup, ie `rsx!{ <span>{count}</span> }`, lowering to
/// the same reactive [`FieldRef`] patch that syncs on `Changed<Document>`.
#[cfg(feature = "scene")]
impl<T>
	crate::prelude::IntoScene<(
		crate::prelude::NotSceneMarker,
		SceneDocStateMarker,
	)> for DocState<T>
where
	T: Default + Serialize + DeserializeOwned + Typed,
{
	fn into_scene(self) -> impl bevy::scene::Scene { self.field().into_scene() }
}

/// System parameter for reading and writing [`DocState`] values.
///
/// Resolves a state's document relative to a subject entity, eg `ev.entity()` in
/// an event handler. Reuses [`DocumentQuery`], so writes are change-detected and
/// schema-checked just like any other document write.
#[derive(SystemParam)]
pub struct DocStateQuery<'w, 's> {
	docs: DocumentQuery<'w, 's>,
}

impl DocStateQuery<'_, '_> {
	/// Read the current value, initializing the field with its default if missing.
	pub fn get<T>(&mut self, subject: Entity, state: &DocState<T>) -> Result<T>
	where
		T: Default + Serialize + DeserializeOwned + Typed,
	{
		let field = state.field();
		self.docs
			.with_field(subject, &field, |value| value.clone())?
			.into_serde()
	}

	/// Write a new value, type-checked against the document schema.
	pub fn set<T>(
		&mut self,
		subject: Entity,
		state: &DocState<T>,
		value: T,
	) -> Result
	where
		T: Default + Serialize + DeserializeOwned + Typed,
	{
		let field = state.field();
		self.docs.set_field_typed(subject, &field, &value)
	}

	/// Read, mutate, and write back in one step.
	pub fn update<T>(
		&mut self,
		subject: Entity,
		state: &DocState<T>,
		func: impl FnOnce(&mut T),
	) -> Result
	where
		T: Default + Serialize + DeserializeOwned + Typed,
	{
		let field = state.field();
		self.docs
			.with_field(subject, &field, move |value| -> Result {
				let mut typed: T = value.clone().into_serde()?;
				func(&mut typed);
				*value = Value::from_serde(&typed)?;
				Ok(())
			})?
	}
}

#[cfg(all(test, feature = "json"))]
mod test {
	use super::*;

	#[beet_core::test]
	fn get_default() {
		let mut world = DocumentPlugin::world();
		let count = DocState::<i64>::new("count").with_default(7);
		let entity = world
			.spawn((Document::default(), children![count.field()]))
			.id();
		world.update_local();

		count
			.get(&mut world.entity_mut(entity))
			.unwrap()
			.xpect_eq(7);
	}

	#[beet_core::test]
	fn set_and_update() {
		let mut world = DocumentPlugin::world();
		let count = DocState::<i64>::new("count").with_default(7);
		let entity = world
			.spawn((Document::default(), children![count.field()]))
			.id();
		world.update_local();

		count.set(&mut world.entity_mut(entity), 10).unwrap();
		world.update_local();
		count
			.get(&mut world.entity_mut(entity))
			.unwrap()
			.xpect_eq(10);

		count
			.update(&mut world.entity_mut(entity), |n| *n += 1)
			.unwrap();
		world.update_local();
		count
			.get(&mut world.entity_mut(entity))
			.unwrap()
			.xpect_eq(11);
	}

	#[beet_core::test]
	fn inline_keys_are_unique() {
		let first = DocState::<u32>::inline();
		let second = DocState::<u32>::inline();
		(first.field().field_path != second.field().field_path).xpect_true();
	}

	#[beet_core::test]
	fn root_document_path() {
		let mut world = DocumentPlugin::world();
		let count = DocState::<i64>::new("count")
			.with_document_path(DocumentPath::Root);
		// state lives on the root, read from a nested child
		let root = world.spawn(Document::default()).id();
		let child = world.spawn(ChildOf(root)).id();
		world.update_local();

		count.set(&mut world.entity_mut(child), 42).unwrap();
		world.update_local();
		// resolves to the same root document from either entity
		count.get(&mut world.entity_mut(root)).unwrap().xpect_eq(42);
	}
}
