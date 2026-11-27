//! Every [`DocPath`] a component holds, found by its type and rewritten where
//! its text sits in the body.
use beet_core::prelude::*;
use bevy::reflect::ReflectRef;

/// Walks a reflected component beside the body value it was read from,
/// calling back for each [`DocPath`] with the value its text sits in, so the
/// resolved path is written where the author wrote the reference.
///
/// The value is followed by the serde shapes a body holds (a struct as a
/// map, a newtype as its field, an option as its content, an enum
/// externally tagged); where the two part ways the reference is still found,
/// with no value to rewrite.
pub(super) struct ReferenceWalk<'a> {
	found: &'a mut dyn FnMut(&DocPath, Option<&mut Value>) -> Result,
}

impl<'a> ReferenceWalk<'a> {
	/// Call `found` for every reference in a walked value.
	pub fn new(
		found: &'a mut dyn FnMut(&DocPath, Option<&mut Value>) -> Result,
	) -> Self {
		Self { found }
	}

	/// Walk `reflected`, read from `value`.
	pub fn walk(
		&mut self,
		reflected: &dyn PartialReflect,
		value: Option<&mut Value>,
	) -> Result {
		if let Some(reflected) = reflected.try_as_reflect() {
			if let Some(path) = reflected.downcast_ref::<DocPath>() {
				// a reference is its text alone
				let value =
					value.filter(|value| matches!(value, Value::Str(_)));
				return (self.found)(path, value);
			}
			// a held template is a document of its own, its references
			// relative to where it was written
			if let Some(held) = reflected.downcast_ref::<HeldTemplate>() {
				for path in held.instances()? {
					(self.found)(&DocPath::from(path), None)?;
				}
				return Ok(());
			}
		}
		match reflected.reflect_ref() {
			ReflectRef::Struct(reflected) => {
				let mut fields =
					value.and_then(|value| value.as_map_mut().ok());
				for index in 0..reflected.field_len() {
					let (Some(field), Some(name)) =
						(reflected.field_at(index), reflected.name_at(index))
					else {
						continue;
					};
					let value = fields
						.as_mut()
						.and_then(|fields| fields.0.get_mut(name));
					self.walk(field, value)?;
				}
			}
			// a newtype is its field
			ReflectRef::TupleStruct(reflected)
				if reflected.field_len() == 1 =>
			{
				if let Some(field) = reflected.field(0) {
					self.walk(field, value)?;
				}
			}
			ReflectRef::TupleStruct(reflected) => {
				let fields = reflected.iter_fields().collect::<Vec<_>>();
				self.walk_items(fields, value)?;
			}
			ReflectRef::Tuple(reflected) => {
				let fields = reflected.iter_fields().collect::<Vec<_>>();
				self.walk_items(fields, value)?;
			}
			ReflectRef::List(reflected) => {
				let items = reflected.iter().collect::<Vec<_>>();
				self.walk_items(items, value)?;
			}
			ReflectRef::Array(reflected) => {
				let items = reflected.iter().collect::<Vec<_>>();
				self.walk_items(items, value)?;
			}
			ReflectRef::Map(reflected) => {
				let mut entries =
					value.and_then(|value| value.as_map_mut().ok());
				for (key, item) in reflected.iter() {
					let key = key
						.try_downcast_ref::<SmolStr>()
						.map(SmolStr::as_str)
						.or_else(|| {
							key.try_downcast_ref::<String>().map(String::as_str)
						});
					let value = entries
						.as_mut()
						.zip(key)
						.and_then(|(entries, key)| entries.0.get_mut(key));
					self.walk(item, value)?;
				}
			}
			ReflectRef::Enum(reflected) => {
				// an option is its content, any other enum externally tagged
				let is_option =
					reflected.get_represented_type_info().is_some_and(|info| {
						info.type_path().starts_with("core::option::Option")
					});
				let payload = match is_option {
					true => value,
					false => value.and_then(|value| {
						value.as_map_mut().ok().and_then(|map| {
							map.0.get_mut(reflected.variant_name())
						})
					}),
				};
				match reflected.field_len() {
					0 => {}
					1 if reflected.name_at(0).is_none() => {
						if let Some(field) = reflected.field_at(0) {
							self.walk(field, payload)?;
						}
					}
					_ => {
						let mut fields =
							payload.and_then(|value| value.as_map_mut().ok());
						for index in 0..reflected.field_len() {
							let Some(field) = reflected.field_at(index) else {
								continue;
							};
							let value = reflected
								.name_at(index)
								.zip(fields.as_mut())
								.and_then(|(name, fields)| {
									fields.0.get_mut(name)
								});
							self.walk(field, value)?;
						}
					}
				}
			}
			_ => {}
		}
		Ok(())
	}

	/// Walk positional `items` beside the list `value` holds.
	fn walk_items(
		&mut self,
		items: Vec<&dyn PartialReflect>,
		value: Option<&mut Value>,
	) -> Result {
		let mut list = value
			.and_then(|value| value.as_list_mut().ok())
			.filter(|list| list.len() == items.len());
		for (index, item) in items.into_iter().enumerate() {
			let value = list.as_mut().and_then(|list| list.get_mut(index));
			self.walk(item, value)?;
		}
		Ok(())
	}
}
