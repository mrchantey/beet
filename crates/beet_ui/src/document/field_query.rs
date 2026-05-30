use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;

/// System parameter for reading and writing document fields.
///
/// Resolves a field's document relative to a subject entity, eg `ev.entity()` in
/// an event handler. Reuses [`DocumentQuery`], so writes are change-detected and
/// schema-checked just like any other document write.
///
/// Two surfaces: an untyped one over raw [`Value`] keyed by [`FieldRef`], and a
/// typed one keyed by [`TypedFieldRef`].
#[derive(SystemParam)]
pub struct FieldQuery<'w, 's> {
	docs: DocumentQuery<'w, 's>,
}

impl FieldQuery<'_, '_> {
	/// Read the current value, initializing the field from its
	/// [`on_missing`](FieldRef::on_missing) if absent.
	pub fn get(&mut self, subject: Entity, field: &FieldRef) -> Result<Value> {
		self.docs.with_field(subject, field, |value| value.clone())
	}

	/// Write a raw value, initializing the field if absent.
	pub fn set(
		&mut self,
		subject: Entity,
		field: &FieldRef,
		value: Value,
	) -> Result {
		self.docs.with_field(subject, field, move |slot| *slot = value)
	}

	/// Read, mutate, and write back a raw value in one step.
	pub fn update(
		&mut self,
		subject: Entity,
		field: &FieldRef,
		func: impl FnOnce(&mut Value),
	) -> Result {
		self.docs.with_field(subject, field, func)
	}

	/// Read the current value as `T`, seeding the default if missing.
	pub fn get_typed<T>(
		&mut self,
		subject: Entity,
		field: &TypedFieldRef<T>,
	) -> Result<T>
	where
		T: DeserializeOwned + Typed,
	{
		self.get(subject, field)?.into_serde()
	}

	/// Write a typed value, type-checked against the document schema.
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

	/// Read, deserialize, mutate, and write back as `T` in one step.
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
	fn untyped_get_set_update() {
		let mut world = DocumentPlugin::world();
		let doc = world.spawn(Document::new(val!({ "count": 1i64 }))).id();
		let field = FieldRef::new("count");

		world
			.run_system_cached_with(
				|In((subject, field)): In<(Entity, FieldRef)>,
				 mut fields: FieldQuery| {
					fields.get(subject, &field).unwrap().xpect_eq(Value::Int(1));
					fields.set(subject, &field, Value::Int(5)).unwrap();
					fields
						.update(subject, &field, |value| {
							*value = Value::Int(value.as_i64().unwrap() + 1)
						})
						.unwrap();
					fields.get(subject, &field).unwrap().xpect_eq(Value::Int(6));
				},
				(doc, field.clone()),
			)
			.unwrap();
	}

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
}
