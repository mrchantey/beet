//! Attributes and literals to the value of the type they name.
use super::lowering::*;
use crate::body::KeySentinels;
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::TypeInfo;
use bevy::reflect::TypeRegistration;
use bevy::reflect::structs::DynamicStruct;

impl Lowering<'_> {
	/// The value of `el`'s attributes as the registered type: its fields
	/// among them, a binding being no field's value.
	pub fn attributes_value(
		&self,
		el: &BsxElement,
		registration: &TypeRegistration,
	) -> Result<Value> {
		let info = match registration.type_info() {
			TypeInfo::Struct(info) => Some(info),
			_ => None,
		};
		let values = self.values();
		let mut keys = KeySentinels::default();
		let mut patch = DynamicStruct::default();
		let fields = info
			.map(|info| {
				el.plain_attributes()
					.filter(|attr| info.field(&attr.key).is_some())
					.filter(|attr| !attr.is_binding())
					.collect::<Vec<_>>()
			})
			.unwrap_or_default();
		for attr in &fields {
			let reflected = attr
				.value
				.literal()
				.and_then(|literal| {
					LiteralReflect::resolve(
						&literal,
						info.and_then(|info| info.field(&attr.key))
							.and_then(|field| field.type_info()),
						self.types,
						&mut values.resolver(&mut keys),
					)
				})
				.map_err(|error| attr.span.locate(error))?;
			patch.insert_boxed(&attr.key, reflected);
		}
		// only a struct target is represented by this patch: a bare tag of any
		// other type is its default, an empty value
		if info.is_none() {
			return match registration.type_info() {
				TypeInfo::Enum(_) | TypeInfo::Opaque(_) => {
					values.default_value(registration)
				}
				_ => Value::List(default()).xok(),
			};
		}
		patch.set_represented_type(Some(registration.type_info()));
		LiteralValues::assert_required(&patch, registration.type_info())?;
		values.patch_value(&patch, keys, || {
			fields
				.iter()
				.filter_map(|attr| {
					attr.value.literal().ok().map(|literal| {
						(
							SmolStr::from(attr.key.as_str()),
							values.literal_to_held(&literal),
						)
					})
				})
				.collect::<Map>()
				.xmap(Value::Map)
		})
	}

	/// Refuse an attribute naming no field of a struct component or resource.
	///
	/// A template forwards the attributes beyond its props, but a component
	/// or a resource has nowhere to forward one.
	pub fn assert_attributes_are_fields(
		el: &BsxElement,
		info: &TypeInfo,
	) -> Result {
		let TypeInfo::Struct(info) = info else {
			return Ok(());
		};
		let unknown = el
			.plain_attributes()
			.filter(|attr| !attr.is_binding())
			.filter(|attr| info.field(&attr.key).is_none())
			.map(|attr| format!("`{}`", attr.key))
			.collect::<Vec<_>>();
		match (unknown.is_empty(), info.field_names()) {
			(true, _) => Ok(()),
			(false, []) => bevybail!(
				"`<{}>` has no field {}: it has none",
				el.tag,
				unknown.join(", ")
			),
			(false, names) => bevybail!(
				"`<{}>` has no field {}: its fields are {}",
				el.tag,
				unknown.join(", "),
				names.join(", ")
			),
		}
	}
}
