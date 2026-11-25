//! Attributes and literals to the value of the type they name.
use super::literal::*;
use super::lowering::*;
use crate::body::KeySentinels;
use crate::bsx::reflect::*;
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::PartialReflect;
use bevy::reflect::TypeInfo;
use bevy::reflect::TypeRegistration;
use bevy::reflect::structs::DynamicStruct;

impl Lowering<'_> {
	/// The registration `name` resolves to: a type by short path, or the enum
	/// a `Type::Variant` names.
	pub fn registration(&self, name: &str) -> Option<&TypeRegistration> {
		registration_by_name(self.types, name)
			.or_else(|| enum_variant_registration(self.types, name))
	}

	/// The value of `named` as the registered type, `held` when no serde form
	/// can write it.
	pub fn named_value(
		&self,
		named: &NamedLiteral,
		registration: &TypeRegistration,
	) -> Result<Value> {
		let mut keys = KeySentinels::default();
		let patch = DataLiteral::to_reflect(
			&DataLiteral::Enum(named.clone()),
			Some(registration.type_info()),
			self.types,
			&mut self.resolver(&mut keys),
		)?;
		Self::assert_required(patch.as_ref(), registration.type_info())?;
		self.patch_value(patch.as_ref(), keys, || self.named_to_held(named))
	}

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
			let reflected = attr_to_literal(&attr.value)
				.and_then(|literal| {
					DataLiteral::to_reflect(
						&literal,
						info.and_then(|info| info.field(&attr.key))
							.and_then(|field| field.type_info()),
						self.types,
						&mut self.resolver(&mut keys),
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
					self.default_value(registration)
				}
				_ => Value::List(default()).xok(),
			};
		}
		patch.set_represented_type(Some(registration.type_info()));
		Self::assert_required(&patch, registration.type_info())?;
		self.patch_value(&patch, keys, || {
			fields
				.iter()
				.filter_map(|attr| {
					attr_to_literal(&attr.value).ok().map(|literal| {
						(
							SmolStr::from(attr.key.as_str()),
							self.literal_to_held(&literal),
						)
					})
				})
				.collect::<Map>()
				.xmap(Value::Map)
		})
	}

	/// The default of a type with no fields to patch.
	pub fn default_value(
		&self,
		registration: &TypeRegistration,
	) -> Result<Value> {
		use bevy::reflect::std_traits::ReflectDefault;
		let default =
			registration.data::<ReflectDefault>().ok_or_else(|| {
				bevyhow!(
					"`{}` has no default, so its tag carries a literal",
					registration.type_info().type_path_table().short_path()
				)
			})?;
		reflect_value_ext::patch_to_value(
			default.default().as_partial_reflect(),
			self.types,
		)
	}

	/// `patch` in its type's serde form, each reference as its key, else as
	/// authored, which a spawn coerces as this did.
	fn patch_value(
		&self,
		patch: &dyn PartialReflect,
		keys: KeySentinels,
		authored: impl FnOnce() -> Value,
	) -> Result<Value> {
		match reflect_value_ext::patch_to_value(patch, self.types) {
			Ok(value) => keys.keyed(value),
			Err(_) => authored().xok(),
		}
	}

	/// Resolve each `$name` to an entity standing for its key.
	fn resolver<'b>(
		&'b self,
		keys: &'b mut KeySentinels,
	) -> impl FnMut(&str) -> Entity + 'b {
		move |name| match self.reference(name) {
			Ok(key) => keys.entity(key),
			Err(err) => {
				warn!("{err}");
				Entity::PLACEHOLDER
			}
		}
	}

	/// Refuse a patch leaving out a `#[reflect(@RequiredField)]` field, which
	/// the fill from the type's default would otherwise mask.
	fn assert_required(patch: &dyn PartialReflect, info: &TypeInfo) -> Result {
		use bevy::reflect::ReflectRef;
		let missing = match (info, patch.reflect_ref()) {
			(TypeInfo::Struct(info), ReflectRef::Struct(patch)) => info
				.iter()
				.find(|field| {
					field.has_attribute::<RequiredField>()
						&& patch.field(field.name()).is_none()
				})
				.map(|field| field.name().to_string()),
			(TypeInfo::TupleStruct(info), ReflectRef::TupleStruct(patch)) => (0
				..info.field_len())
				.find(|index| {
					info.field_at(*index).is_some_and(|field| {
						field.has_attribute::<RequiredField>()
					}) && patch.field(*index).is_none()
				})
				.map(|index| index.to_string()),
			_ => None,
		};
		match missing {
			Some(field) => bevybail!(
				"{}: missing required field '{field}'",
				info.type_path_table().short_path()
			),
			None => Ok(()),
		}
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
