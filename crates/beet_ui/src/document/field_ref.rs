use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;
use core::marker::PhantomData;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;



/// A reference to a specific field in a document.
///
/// Used by content and actions to interact with document fields. By default,
/// fields are initialized with `null` if missing, unless configured otherwise
/// via [`on_missing`](FieldRef::on_missing).
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
	Get,
	SetWith,
)]
#[reflect(Component)]
#[component(immutable, on_add=on_add)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FieldRef {
	/// The path to the document
	pub document: DocumentPath,
	/// The path to the field within the document
	pub field_path: FieldPath,
	/// Behavior when the field is missing from the document.
	pub on_missing: OnMissingField,
}

fn on_add(mut world: DeferredWorld, cx: HookContext) {
	if !world.entity(cx.entity).contains::<Value>() {
		let this = world.entity(cx.entity).get::<FieldRef>().unwrap();
		let value = match this.on_missing.clone() {
			OnMissingField::Init { value } => value,
			_ => Value::default(),
		};
		world.commands().entity(cx.entity).insert(value);
	}
}

impl core::fmt::Display for FieldRef {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "{}#{}", self.document, self.field_path)
	}
}


impl FieldRef {
	/// Create a new field reference with the given field path.
	///
	/// Uses the default [`DocumentPath::Ancestor`] for document resolution.
	/// Use [`with_document`](Self::with_document) to specify a different document.
	///
	/// By default, missing fields are initialized with [`Value::Null`].
	pub fn new<M>(field_path: impl IntoFieldPath<M>) -> Self {
		Self {
			document: DocumentPath::default(),
			field_path: field_path.into_field_path(),
			on_missing: OnMissingField::default(),
		}
	}

	/// Set this field reference to error if the field is missing instead of initializing it.
	pub fn error_on_missing(mut self) -> Self {
		self.on_missing = OnMissingField::EmitError;
		self
	}

	/// Set the field to initialize with a specific value if missing.
	pub fn with_init(mut self, value: impl Into<Value>) -> Self {
		self.on_missing = OnMissingField::Init {
			value: value.into(),
		};
		self
	}
}

/// A typed, keyed handle to a document field, Beet's atom.
///
/// A thin newtype over [`FieldRef`] that derefs to the inner ref, so all the
/// [`FieldRef`] builders and [`Get`] accessors work through deref. The typed
/// layer adds one behavior: the field is seeded with `T::default()` on first
/// touch (via [`OnMissingField::Init`]) and reads/writes go through `T` with
/// [`FieldQuery`].
///
/// ## Placement is explicit
///
/// Widgets never auto-scope a [`Document`]. The author decides where state lives:
/// - explicit keys ([`TypedFieldRef::new`]) are shared, named atoms, one value
///   per document, ie a Recoil-style atom that other code can target by key.
/// - per-instance state uses [`TypedFieldRef::inline`] (a process-unique key)
///   **or** a dedicated ancestor [`Document`] per instance.
///
/// Two refs with the *same* explicit key in the *same* document collide; that is
/// an author error to resolve with a distinct key,
/// [`inline`](TypedFieldRef::inline), or a separate document.
#[derive(Deref, DerefMut)]
pub struct TypedFieldRef<T> {
	#[deref]
	field: FieldRef,
	/// The field's resolved schema, ie `ValueSchema::of::<T>()`, computed once at
	/// construction and co-located on the spawned field entity by [`field`](Self::field).
	schema: ValueSchema,
	_marker: PhantomData<fn() -> T>,
}

// Hand-written so `T` is never constrained: the `PhantomData<fn() -> T>` means
// the stored data is always `Clone` + `Debug` regardless of `T`.
impl<T> Clone for TypedFieldRef<T> {
	fn clone(&self) -> Self {
		Self {
			field: self.field.clone(),
			schema: self.schema.clone(),
			_marker: PhantomData,
		}
	}
}

impl<T> core::fmt::Debug for TypedFieldRef<T> {
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		f.debug_struct("TypedFieldRef")
			.field("field", &self.field)
			.field("schema", &self.schema)
			.finish()
	}
}

impl<T> TypedFieldRef<T>
where
	T: Default + Serialize + Typed,
{
	/// Declare a typed field ref with an explicit, shared key, ie a named atom.
	///
	/// The field is seeded with `T::default()` on first touch. The key is the
	/// contract other code targets by name; collisions on the same key in the
	/// same document are an author error.
	pub fn new<M>(key: impl IntoFieldPath<M>) -> Self {
		Self {
			field: FieldRef::new(key).with_init(
				Value::from_serde(&T::default()).unwrap_or_default(),
			),
			schema: ValueSchema::of::<T>(),
			_marker: PhantomData,
		}
	}

	/// Declare a typed field ref with a process-unique `"inline_{N}"` key.
	///
	/// Use this for state you want tracked inside this specific context but that
	/// nothing else will ever read by name. The key is an implementation detail;
	/// no other code cares what its address is. Contrast [`new`](Self::new), a
	/// shared named atom that other code can target by key.
	pub fn inline() -> Self {
		/// Process-global counter backing [`TypedFieldRef::inline`] keys.
		static INLINE_KEY_COUNTER: AtomicUsize = AtomicUsize::new(0);
		let index = INLINE_KEY_COUNTER.fetch_add(1, Ordering::Relaxed);
		Self::new(format!("inline_{index}"))
	}
}

impl<T: Typed> TypedFieldRef<T> {
	/// Re-type an existing [`FieldRef`] as a [`TypedFieldRef<T>`].
	///
	/// For places that hold an erased [`FieldRef`] (ie read from a component)
	/// but know its `T`, so typed reads/writes can go through [`FieldQuery`].
	pub fn from_field(field: FieldRef) -> Self {
		Self {
			field,
			schema: ValueSchema::of::<T>(),
			_marker: PhantomData,
		}
	}
}

impl<T> TypedFieldRef<T> {
	/// The [`FieldRef`] paired with its co-located [`ValueSchema`], for places
	/// that need an owned bundle rather than a deref borrow, ie
	/// `ReactiveChildren::new(items.field(), ..)` or a `children![items.field()]`
	/// markup slot. The schema lets a typed write type-check locally.
	pub fn field(&self) -> (FieldRef, ValueSchema) {
		(self.field.clone(), self.schema.clone())
	}

	/// Map the inner [`FieldRef`], preserving the typed wrapper.
	///
	/// The [`FieldRef`] builders consume `self` by value, so they can't chain
	/// through `Deref`; the named builders below route through this. The `Get`
	/// accessors (`field_path()`, `document()`, `on_missing()`) take `&self` and
	/// do work through deref, so they are not duplicated.
	fn map_field(mut self, func: impl FnOnce(FieldRef) -> FieldRef) -> Self {
		self.field = func(self.field);
		self
	}

	/// Resolve against a specific [`DocumentPath`] instead of the default
	/// [`DocumentPath::Ancestor`].
	pub fn with_document(self, document: DocumentPath) -> Self {
		self.map_field(|field| field.with_document(document))
	}

	/// Override the value seeded when the field is missing.
	pub fn with_init(self, value: impl Into<Value>) -> Self {
		self.map_field(|field| field.with_init(value))
	}

	/// Error instead of seeding when the field is missing.
	pub fn error_on_missing(self) -> Self {
		self.map_field(FieldRef::error_on_missing)
	}
}

impl<T> TypedFieldRef<T>
where
	T: Default + Serialize + DeserializeOwned + Typed,
{
	/// Read the current value against `entity`, seeding the default if missing.
	pub fn get(&self, entity: &mut EntityWorldMut) -> Result<T> {
		entity.with_state::<FieldQuery, _>(|subject, mut fields| {
			fields.get_typed(subject, self)
		})
	}

	/// Write a new value against `entity`, type-checked against the schema.
	pub fn set(&self, entity: &mut EntityWorldMut, value: T) -> Result {
		entity.with_state::<FieldQuery, _>(|subject, mut fields| {
			fields.set_typed(subject, self, value)
		})
	}

	/// Read, mutate, and write back in one step against `entity`.
	pub fn update(
		&self,
		entity: &mut EntityWorldMut,
		func: impl FnOnce(&mut T),
	) -> Result {
		entity.with_state::<FieldQuery, _>(|subject, mut fields| {
			fields.update_typed(subject, self, func)
		})
	}
}

/// Marker disambiguating the [`TypedFieldRef`] markup-read [`IntoScene`] impl.
#[cfg(feature = "scene")]
pub struct SceneTypedFieldRefMarker;

/// Read a [`TypedFieldRef`] in markup, ie `rsx!{ <span>{count}</span> }`,
/// lowering to the inner [`FieldRef`] that syncs on `Changed<Document>`.
#[cfg(feature = "scene")]
impl<T> crate::prelude::IntoScene<(NotSceneMarker, SceneTypedFieldRefMarker)>
	for TypedFieldRef<T>
{
	fn into_scene(self) -> impl bevy::scene::Scene {
		(self.field, self.schema).into_scene()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use core::ops::Deref;

	#[beet_core::test]
	fn field_ref_new() {
		let field = FieldRef::new("field");

		field.document.xpect_eq(DocumentPath::Ancestor);
		field
			.field_path
			.deref()
			.xpect_eq(vec![FieldSegment::key("field")]);
		field
			.on_missing
			.xpect_eq(OnMissingField::Init { value: Value::Null });
	}

	#[beet_core::test]
	fn inline_keys_are_unique() {
		let first = TypedFieldRef::<u32>::inline();
		let second = TypedFieldRef::<u32>::inline();
		(first.field_path != second.field_path).xpect_true();
	}

	#[cfg(feature = "json")]
	#[beet_core::test]
	fn seeds_default_and_round_trips() {
		let mut world = DocumentPlugin::world();
		let count = TypedFieldRef::<i64>::new("count").with_init(7);
		// the field entity carries its own (Value, ValueSchema): the local fast path
		let doc = world.spawn(Document::default()).id();
		let field = world.spawn((ChildOf(doc), count.field())).id();
		// settle the seed so the document's changed flag ages out, else the read
		// path would clobber the local edits when they are mirrored back
		world.update_local();
		world.update_local();

		// the schema lands on the field entity alongside the seeded value
		world
			.entity(field)
			.get::<ValueSchema>()
			.unwrap()
			.clone()
			.xpect_eq(ValueSchema::of::<i64>());

		// reads come straight off the local Value, no document traversal
		count.get(&mut world.entity_mut(field)).unwrap().xpect_eq(7);

		// a typed write hits the local Value immediately, observable before sync
		count.set(&mut world.entity_mut(field), 10).unwrap();
		count.get(&mut world.entity_mut(field)).unwrap().xpect_eq(10);

		// update reads, mutates and writes the local Value in one step
		count
			.update(&mut world.entity_mut(field), |n| *n += 1)
			.unwrap();
		count.get(&mut world.entity_mut(field)).unwrap().xpect_eq(11);

		// bidi sync mirrors the local Value into the document
		world.update_local();
		world
			.entity(doc)
			.get::<Document>()
			.unwrap()
			.get_field::<i64>(&[FieldSegment::key("count")])
			.unwrap()
			.xpect_eq(11);
	}

	#[cfg(feature = "json")]
	#[beet_core::test]
	fn root_document_path() {
		let mut world = DocumentPlugin::world();
		let count =
			TypedFieldRef::<i64>::new("count").with_document(DocumentPath::Root);
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
