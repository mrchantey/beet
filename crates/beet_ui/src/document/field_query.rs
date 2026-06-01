use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;

/// System parameter for reading and writing document fields.
///
/// Resolves a field's document relative to a subject entity, eg `ev.entity()` in
/// an event handler. Reuses [`DocumentQuery`], so writes are change-detected and
/// schema-checked just like any other document write.
///
/// This is the schema-checked typed surface, keyed by [`TypedFieldRef`]. A
/// self-bound subject already mirrors its field in a local [`Value`], so untyped
/// reads/writes go straight through that `Value` rather than this indirection.
#[derive(SystemParam)]
pub struct FieldQuery<'w, 's> {
	docs: DocumentQuery<'w, 's>,
}

impl FieldQuery<'_, '_> {
	/// Read the current value as `T`, seeding the default if missing.
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
