//! An authored literal to the value a scene body holds, the half of a lowering
//! every authored source shares: markup's spreads and tags, a document's
//! frontmatter, an office file's metadata.
use super::*;
use crate::body::KeySentinels;
use beet_core::prelude::*;
use bevy::reflect::PartialReflect;
use bevy::reflect::TypeInfo;
use bevy::reflect::TypeRegistration;
use bevy::reflect::TypeRegistry;

/// Resolves the `$name` a literal references to the key of the entity it
/// names, which only a format with entities of its own can answer.
pub trait LiteralReferences {
	/// The key `$name` references.
	fn reference(&self, name: &str) -> Result<EntityKey>;
}

/// A source naming no entities, so any `$name` in it is an error.
pub struct NoReferences;

impl LiteralReferences for NoReferences {
	fn reference(&self, name: &str) -> Result<EntityKey> {
		bevybail!("`${name}` references an entity, and nothing here names one")
	}
}

/// Authored literals resolved against the types a document lowers against:
/// each a value in its type's serde form where one is registered, else as
/// authored, so a spawn holds it in [`Unregistered`].
pub struct LiteralValues<'a> {
	/// The types names resolve against.
	pub types: &'a TypeRegistry,
	/// What each `$name` references.
	pub references: &'a dyn LiteralReferences,
}

impl<'a> LiteralValues<'a> {
	/// Resolve against `types`, a `$name` referencing nothing.
	pub fn new(types: &'a TypeRegistry) -> Self {
		Self {
			types,
			references: &NoReferences,
		}
	}

	/// `declarations` as the components of one entity, as markup's spreads on
	/// it would be: a registered type under its full path in its serde form, a
	/// name nothing registered as authored.
	///
	/// ## Errors
	/// Errors when a literal does not fit the type it names.
	pub fn declarations(
		&self,
		declarations: impl IntoIterator<Item = NamedLiteral>,
	) -> Result<EntityBody> {
		let mut body = EntityBody::default();
		for named in declarations {
			let (name, value) = self.declaration(&named)?;
			body.insert(name, value);
		}
		body.xok()
	}

	/// One declaration: the name its value is keyed by and the value.
	pub fn declaration(
		&self,
		named: &NamedLiteral,
	) -> Result<(SmolStr, Value)> {
		let Some(registration) = self
			.registration(&named.name)
			.filter(|registration| NameKind::of(registration).is_some())
		else {
			return (named.name.clone(), self.named_to_held(named)).xok();
		};
		(
			SmolStr::new(registration.type_info().type_path()),
			self.named_value(named, registration)?,
		)
			.xok()
	}

	/// The registration `name` resolves to: a type by short or full path, or
	/// the enum a `Type::Variant` names.
	pub fn registration(&self, name: &str) -> Option<&'a TypeRegistration> {
		reflect_ext::registration_by_name(self.types, name)
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
		let patch = LiteralReflect::resolve(
			&DataLiteral::Enum(named.clone()),
			Some(registration.type_info()),
			self.types,
			&mut self.resolver(&mut keys),
		)?;
		Self::assert_required(patch.as_ref(), registration.type_info())?;
		self.patch_value(patch.as_ref(), keys, || self.named_to_held(named))
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
	pub(crate) fn patch_value(
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
	pub(crate) fn resolver<'b>(
		&'b self,
		keys: &'b mut KeySentinels,
	) -> impl FnMut(&str) -> Entity + 'b {
		move |name| match self.references.reference(name) {
			Ok(key) => keys.entity(key),
			Err(err) => {
				warn!("{err}");
				Entity::PLACEHOLDER
			}
		}
	}

	/// Refuse a patch leaving out a `#[reflect(@RequiredField)]` field, which
	/// the fill from the type's default would otherwise mask.
	pub fn assert_required(
		patch: &dyn PartialReflect,
		info: &TypeInfo,
	) -> Result {
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

	/// The value an authored literal stands for when no type says more, in the
	/// shape its serde form would take: an enum variant by name with any fields
	/// under it, a reference its key.
	pub fn literal_to_held(&self, literal: &DataLiteral) -> Value {
		match literal {
			DataLiteral::Scalar(value) => value.clone(),
			DataLiteral::List(items) => items
				.iter()
				.map(|item| self.literal_to_held(item))
				.collect::<Vec<_>>()
				.xmap(Value::List),
			DataLiteral::Struct(fields) => self.fields_to_held(fields),
			DataLiteral::Enum(named) => match &named.fields {
				NamedFields::Unit => Value::Str(named.name.clone()),
				_ => Value::Map(Map::new([(
					named.name.clone(),
					self.named_to_held(named),
				)])),
			},
			DataLiteral::EntityRef(name) => self
				.references
				.reference(name)
				.map(|key| key.reference())
				.unwrap_or_else(|_| Value::str(format!("${name}"))),
			DataLiteral::Binding(_) => Value::Null,
		}
	}

	/// The fields of a named literal as the value its type would hold: a
	/// struct its fields by name, a single positional field that field,
	/// several a list.
	pub fn named_to_held(&self, named: &NamedLiteral) -> Value {
		match &named.fields {
			NamedFields::Unit => Value::map(),
			NamedFields::Tuple(items) => match items.as_slice() {
				[item] => self.literal_to_held(item),
				items => items
					.iter()
					.map(|item| self.literal_to_held(item))
					.collect::<Vec<_>>()
					.xmap(Value::List),
			},
			NamedFields::Struct(fields) => self.fields_to_held(fields),
		}
	}

	fn fields_to_held(&self, fields: &[(SmolStr, DataLiteral)]) -> Value {
		fields
			.iter()
			.map(|(key, item)| (key.clone(), self.literal_to_held(item)))
			.collect::<Map>()
			.xmap(Value::Map)
	}

	/// A literal in text or attribute position as a plain [`Value`], refusing
	/// what only a type gives meaning: an enum with fields, a reference, a
	/// binding.
	pub fn plain(literal: &DataLiteral) -> Result<Value> {
		match literal {
			DataLiteral::Scalar(value) => Ok(value.clone()),
			DataLiteral::List(items) => items
				.iter()
				.map(Self::plain)
				.collect::<Result<Vec<_>>>()
				.map(Value::List),
			DataLiteral::Struct(fields) => {
				let mut map = Map::default();
				for (key, value) in fields {
					map.insert(key.clone(), Self::plain(value)?);
				}
				Ok(Value::Map(map))
			}
			DataLiteral::Enum(named)
				if matches!(named.fields, NamedFields::Unit) =>
			{
				Ok(Value::Str(named.name.clone()))
			}
			DataLiteral::Enum(_) => {
				bevybail!("enum literal with fields is not a plain text value")
			}
			DataLiteral::EntityRef(_) => {
				bevybail!(
					"`$name` entity references are not a plain text value"
				)
			}
			DataLiteral::Binding(_) => {
				bevybail!("an `@` binding inside a literal is no plain value")
			}
		}
	}
}

/// What a registered name resolves to on an entity.
pub(crate) enum NameKind {
	/// A `#[template]` type, expanded when the body spawns.
	Template,
	/// A resource declaration.
	Resource,
	/// A component.
	Component,
}

impl NameKind {
	/// What `registration` is, `None` for a type that is none of the three:
	/// registered by reflection alone (bevy's `reflect_auto_register`, or a
	/// template whose installer this build never ran), it means nothing on an
	/// entity and is held as any unregistered name is, so the document still
	/// loads whole.
	pub fn of(registration: &TypeRegistration) -> Option<Self> {
		if registration.data::<ReflectTemplate>().is_some() {
			Some(Self::Template)
		} else if registration.data::<ReflectResource>().is_some() {
			Some(Self::Resource)
		} else if registration.data::<ReflectComponent>().is_some() {
			Some(Self::Component)
		} else {
			None
		}
	}
}
