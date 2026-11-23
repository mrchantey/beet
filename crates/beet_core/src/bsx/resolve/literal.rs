//! Leaf lowering: an attribute or text literal to the concrete [`Value`] or
//! [`DataLiteral`] the layer above patches with.

use super::directives::*;
use crate::prelude::*;

/// Lower a literal in text/attribute position to a concrete [`Value`].
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
			Ok(Value::Str(named.name.clone().into()))
		}
		DataLiteral::Enum(_) => {
			bevybail!("enum literal with fields is not a plain text value")
		}
		DataLiteral::EntityRef(_) => {
			bevybail!("`$name` entity references are not a plain text value")
		}
	}
}

/// A literal as the value an [`Unregistered`] holds for a component nothing
/// resolved, in the shape its serde form would take: an enum variant by name,
/// with any fields under it. Total, since nothing is known of the type: a
/// `$name` entity reference is held as it was written.
pub(super) fn literal_to_held(literal: &DataLiteral) -> Value {
	match literal {
		DataLiteral::Scalar(value) => value.clone(),
		DataLiteral::List(items) => {
			Value::List(items.iter().map(literal_to_held).collect())
		}
		DataLiteral::Struct(fields) => fields_to_held(fields),
		DataLiteral::Enum(named) => match &named.fields {
			NamedFields::Unit => Value::Str(named.name.clone()),
			_ => Value::Map(Map::new([(
				named.name.clone(),
				named_to_held(named),
			)])),
		},
		DataLiteral::EntityRef(name) => Value::str(format!("${name}")),
	}
}

/// The fields of a named literal as the value its type would hold: a struct
/// its fields by name, a single positional field that field, several a list.
pub(super) fn named_to_held(named: &NamedLiteral) -> Value {
	match &named.fields {
		NamedFields::Unit => Value::map(),
		NamedFields::Tuple(items) => match items.as_slice() {
			[item] => literal_to_held(item),
			items => Value::List(items.iter().map(literal_to_held).collect()),
		},
		NamedFields::Struct(fields) => fields_to_held(fields),
	}
}

/// A tag's literal attributes as the fields of the component it names. A
/// directive, a spread and an `@` binding are not fields, and each applies to
/// the entity through its own machinery.
pub(super) fn attributes_to_held(el: &BsxElement) -> Value {
	el.attributes
		.iter()
		.filter(|attr| !is_directive(&attr.key) && !attr.key.is_empty())
		.filter_map(|attr| {
			attr_to_literal(&attr.value).ok().map(|literal| {
				(attr.key.clone().into(), literal_to_held(&literal))
			})
		})
		.collect::<Map>()
		.xmap(Value::Map)
}

fn fields_to_held(fields: &[(SmolStr, DataLiteral)]) -> Value {
	fields
		.iter()
		.map(|(key, item)| (key.clone(), literal_to_held(item)))
		.collect::<Map>()
		.xmap(Value::Map)
}

/// Lower an attribute value to a literal for reflect-patching an uppercase tag.
pub(super) fn attr_to_literal(value: &AttrValue) -> Result<DataLiteral> {
	match value {
		AttrValue::Flag => Ok(DataLiteral::Scalar(Value::Bool(true))),
		AttrValue::Str(string) => {
			Ok(DataLiteral::Scalar(Value::Str(string.into())))
		}
		AttrValue::Expr(ValueExpr::Literal(literal)) => Ok(literal.clone()),
		// `field=$name` lowers to an entity-reference literal on the patched field.
		AttrValue::Expr(ValueExpr::EntityRef(name)) => {
			Ok(DataLiteral::EntityRef(name.clone()))
		}
		AttrValue::Expr(ValueExpr::Binding(_)) => {
			bevybail!("an `@` binding is not a component patch value")
		}
		AttrValue::Spread(_) => bevybail!("a spread is not an attribute value"),
	}
}

// --- attribute lookup helpers ------------------------------------------------

/// The string value of a literal-string attribute, if present.
pub(super) fn string_attr(el: &BsxElement, key: &str) -> Option<String> {
	el.attributes.iter().find_map(|attr| {
		if attr.key != key {
			return None;
		}
		match &attr.value {
			AttrValue::Str(string) => Some(string.clone()),
			AttrValue::Flag => Some(String::new()),
			_ => None,
		}
	})
}
