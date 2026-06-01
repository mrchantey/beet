use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;

/// System parameter for reading and writing document fields, the schema-checked
/// typed surface keyed by [`TypedFieldRef`].
///
/// Two families of methods, by how the field is addressed:
///
/// - **local** ([`get_local`](Self::get_local) / [`set_local`](Self::set_local)
///   / [`update_local`](Self::update_local)): the entity *is* the field, ie it
///   carries its own [`Value`] and co-located [`ValueSchema`] (as every
///   [`TypedFieldRef::field`] bundle does). One archetype lookup, a leaf
///   type-check, no document traversal. The mutation lands on the local `Value`
///   and bidi sync mirrors it into the document, exactly as the untyped
///   self-bound actions do. No [`TypedFieldRef`] needed, only the entity and `T`.
///
/// - **traversal** ([`get_typed`](Self::get_typed) /
///   [`set_typed`](Self::set_typed) / [`update_typed`](Self::update_typed)):
///   resolve the field's document via [`DocumentQuery`] by [`DocumentPath`] and
///   descend by path. Keyed by a [`TypedFieldRef`], for cross-entity refs (eg
///   `with_document(Root)`) and any subject with no local mirror.
///
/// [`TypedFieldRef::get`]/`set`/`update` pick the local path when the subject
/// [`is_local`](Self::is_local) and fall back to traversal otherwise.
#[derive(SystemParam)]
pub struct FieldQuery<'w, 's> {
	/// Field entities carrying their own value and schema, ie the fast path.
	local: Query<'w, 's, (&'static mut Value, &'static ValueSchema)>,
	docs: DocumentQuery<'w, 's>,
}

impl FieldQuery<'_, '_> {
	/// Whether `entity` is a self-bound field, ie it carries its own [`Value`]
	/// and [`ValueSchema`] and so takes the local fast path.
	pub fn is_local(&self, entity: Entity) -> bool {
		self.local.contains(entity)
	}

	/// Read the field entity's own [`Value`] as `T`, no document traversal.
	pub fn get_local<T>(&self, entity: Entity) -> Result<T>
	where
		T: DeserializeOwned + Typed,
	{
		let (value, _schema) = self.local.get(entity)?;
		value.clone().into_serde()
	}

	/// Write `value` to the field entity's own [`Value`], type-checked against its
	/// co-located [`ValueSchema`]. Bidi sync mirrors it to the document.
	pub fn set_local<T>(&mut self, entity: Entity, value: T) -> Result
	where
		T: Serialize + Typed,
	{
		let (mut slot, schema) = self.local.get_mut(entity)?;
		schema.assert_matches(&ValueSchema::of::<T>(), &[])?;
		*slot = Value::from_serde(&value)?;
		Ok(())
	}

	/// Read, mutate and write back the field entity's own [`Value`] as `T` in one
	/// step, type-checked against its co-located [`ValueSchema`].
	pub fn update_local<T>(
		&mut self,
		entity: Entity,
		func: impl FnOnce(&mut T),
	) -> Result
	where
		T: Serialize + DeserializeOwned + Typed,
	{
		let (mut slot, schema) = self.local.get_mut(entity)?;
		schema.assert_matches(&ValueSchema::of::<T>(), &[])?;
		let mut typed: T = slot.clone().into_serde()?;
		func(&mut typed);
		*slot = Value::from_serde(&typed)?;
		Ok(())
	}

	/// Resolve `field`'s document from `subject` and read it as `T`, seeding the
	/// default if missing.
	pub fn get_typed<T>(
		&mut self,
		subject: Entity,
		field: &TypedFieldRef<T>,
	) -> Result<T>
	where
		T: DeserializeOwned + Typed,
	{
		self.docs
			.with_field(subject, field, |value| value.clone())?
			.into_serde()
	}

	/// Resolve `field`'s document from `subject` and write `value`, type-checked
	/// against the document schema.
	pub fn set_typed<T>(
		&mut self,
		subject: Entity,
		field: &TypedFieldRef<T>,
		value: T,
	) -> Result
	where
		T: Serialize + Typed,
	{
		self.docs.set_field_typed(subject, field, &value)
	}

	/// Resolve `field`'s document from `subject` and read, mutate and write it
	/// back as `T` in one step.
	pub fn update_typed<T>(
		&mut self,
		subject: Entity,
		field: &TypedFieldRef<T>,
		func: impl FnOnce(&mut T),
	) -> Result
	where
		T: Serialize + DeserializeOwned + Typed,
	{
		self.docs
			.with_field(subject, field, move |value| -> Result {
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
	fn typed_seeds_default() {
		let mut world = DocumentPlugin::world();
		let doc = world.spawn(Document::default()).id();
		let count = TypedFieldRef::<i64>::new("count");

		world
			.run_system_cached_with(
				|In((subject, count)): In<(Entity, TypedFieldRef<i64>)>,
				 mut fields: FieldQuery| {
					// missing field seeded with T::default()
					fields.get_typed(subject, &count).unwrap().xpect_eq(0);
					fields.set_typed(subject, &count, 10).unwrap();
					fields
						.update_typed(subject, &count, |n| *n += 1)
						.unwrap();
					fields.get_typed(subject, &count).unwrap().xpect_eq(11);
				},
				(doc, count),
			)
			.unwrap();
	}

	#[beet_core::test]
	fn local_write_rejects_wrong_type() {
		let mut world = DocumentPlugin::world();
		let count = TypedFieldRef::<i64>::new("count");
		// the field entity carries an i64 schema via `count.field()`
		let doc = world.spawn(Document::default()).id();
		let field = world.spawn((ChildOf(doc), count.field())).id();
		world.update_local();

		// a String write contradicts the field's local i64 schema
		world
			.run_system_cached_with(
				|In(subject): In<Entity>, mut fields: FieldQuery| {
					fields
						.set_local::<String>(subject, "oops".to_string())
						.is_err()
						.xpect_true();
				},
				field,
			)
			.unwrap();
	}
}
