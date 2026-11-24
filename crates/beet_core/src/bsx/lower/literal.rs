//! Leaf lowering: an attribute or text literal to the [`Value`] or
//! [`DataLiteral`] it stands for.
use super::lowering::*;
use crate::prelude::*;

/// Lower a literal in text or attribute position to a concrete [`Value`].
pub(in crate::bsx) fn literal_to_value(literal: &DataLiteral) -> Result<Value> {
	match literal {
		DataLiteral::Scalar(value) => Ok(value.clone()),
		DataLiteral::List(items) => items
			.iter()
			.map(literal_to_value)
			.collect::<Result<Vec<_>>>()
			.map(Value::List),
		DataLiteral::Struct(fields) => {
			let mut map = Map::default();
			for (key, value) in fields {
				map.insert(key.clone(), literal_to_value(value)?);
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
			bevybail!("`$name` entity references are not a plain text value")
		}
		DataLiteral::Binding(_) => {
			bevybail!("an `@` binding inside a literal is no plain value")
		}
	}
}

/// Lower an attribute value to a literal for patching the field it names.
pub(super) fn attr_to_literal(value: &AttrValue) -> Result<DataLiteral> {
	match value {
		AttrValue::Flag => Ok(DataLiteral::Scalar(Value::Bool(true))),
		AttrValue::Str(string) => {
			Ok(DataLiteral::Scalar(Value::Str(string.into())))
		}
		AttrValue::Expr(ValueExpr::Literal(literal)) => Ok(literal.clone()),
		// `field=$name` lowers to an entity-reference literal on the field
		AttrValue::Expr(ValueExpr::EntityRef(name)) => {
			Ok(DataLiteral::EntityRef(name.clone()))
		}
		AttrValue::Expr(ValueExpr::Binding(_)) => {
			bevybail!("an `@` binding is not a component patch value")
		}
		AttrValue::Spread(_) => bevybail!("a spread is not an attribute value"),
	}
}

/// The value an authored literal stands for when no type says more, in the
/// shape its serde form would take: an enum variant by name with any fields
/// under it, a reference its key.
impl Lowering<'_> {
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

	/// A tag's literal attributes as the fields of the component it names. A
	/// spread and an `@` binding are not fields, each applying to the entity
	/// through its own machinery.
	pub fn attributes_to_held(&self, el: &BsxElement) -> Value {
		el.plain_attributes()
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
	}

	fn fields_to_held(&self, fields: &[(SmolStr, DataLiteral)]) -> Value {
		fields
			.iter()
			.map(|(key, item)| (key.clone(), self.literal_to_held(item)))
			.collect::<Map>()
			.xmap(Value::Map)
	}
}
