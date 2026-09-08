//! Leaf lowering: an attribute or text literal to the concrete [`Value`] or
//! [`DataLiteral`] the layer above patches with.

use crate::prelude::*;

/// Lower a literal in text/attribute position to a concrete [`Value`].
pub(super) fn literal_to_value(literal: &DataLiteral) -> Result<Value> {
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
		AttrValue::Style { .. } => {
			bevybail!("a `bx:style` directive is not a component patch value")
		}
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
