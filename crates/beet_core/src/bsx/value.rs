//! The BSX value grammar: literals, `#`/`$` references, and spreads.
//!
//! A hand-written cursor over the value surface, mirroring the `bsn!` surface:
//! scalars, named-field structs, lists, and enums (unit/tuple/struct variants),
//! plus `#field` document references and `$entity` references. Position decides
//! which entry point is called: [`parse_value_expr`] for attribute-value and
//! text position, [`parse_spread`] for bare-child position. Both share the
//! literal grammar ([`parse_literal`]).

use super::ast::*;
use super::cursor::Cursor;
use crate::prelude::*;

/// Parse an attribute-value or text-position value expression from a string, the
/// entry a non-cursor caller (eg the markdown front-end) uses for a `{..}` block.
pub fn parse_value_expr_str(source: &str) -> Result<ValueExpr> {
	parse_value_expr(&mut Cursor::new(source))
}

/// Parse an attribute-value or text-position value: a literal or a reference.
pub fn parse_value_expr(cursor: &mut Cursor) -> Result<ValueExpr> {
	cursor.skip_ws();
	match cursor.peek() {
		Some('#') => parse_field_ref(cursor),
		Some('$') => parse_entity_ref(cursor),
		_ => parse_literal(cursor).map(ValueExpr::Literal),
	}
}

/// Parse a bare-position spread `{MyComponent{..}}` or `{(A, B)}`.
pub fn parse_spread(cursor: &mut Cursor) -> Result<SpreadExpr> {
	cursor.skip_ws();
	if cursor.peek() == Some('(') {
		cursor.bump();
		let mut items = Vec::new();
		loop {
			cursor.skip_ws();
			if cursor.eat(")") {
				break;
			}
			items.push(parse_named_literal(cursor)?);
			cursor.skip_ws();
			if !cursor.eat(",") && cursor.peek() != Some(')') {
				bevybail!("expected `,` or `)` in spread tuple");
			}
		}
		Ok(SpreadExpr::Tuple(items))
	} else {
		parse_named_literal(cursor).map(SpreadExpr::Named)
	}
}

/// Parse a single literal: scalar, list, struct, or enum.
pub fn parse_literal(cursor: &mut Cursor) -> Result<DataLiteral> {
	cursor.skip_ws();
	match cursor.peek() {
		Some('"') => Ok(DataLiteral::Scalar(Value::Str(parse_string(cursor)?))),
		Some('[') => parse_list(cursor),
		Some('{') => parse_struct(cursor).map(DataLiteral::Struct),
		Some('$') => {
			// a `$name` entity reference nested in a literal (eg a spread field).
			cursor.eat("$");
			let name = cursor.take_while(|ch| ch.is_alphanumeric() || ch == '_');
			if name.is_empty() {
				bevybail!("expected an entity name after `$`");
			}
			Ok(DataLiteral::EntityRef(name.into()))
		}
		Some(ch) if ch.is_ascii_digit() || ch == '-' || ch == '+' => {
			parse_number(cursor)
		}
		Some(ch) if ch.is_alphabetic() || ch == '_' => parse_ident_value(cursor),
		other => bevybail!("unexpected character in value: {other:?}"),
	}
}

/// Parse a `#field.path` reference with an optional `=init` initializer.
fn parse_field_ref(cursor: &mut Cursor) -> Result<ValueExpr> {
	cursor.eat("#");
	let path = parse_field_path(cursor)?;
	// an optional `=literal` initializer.
	let init = if cursor.peek() == Some('=') {
		cursor.bump();
		Some(parse_literal(cursor)?)
	} else {
		None
	};
	Ok(ValueExpr::FieldRef { path, init })
}

/// Parse a `$name` entity reference.
fn parse_entity_ref(cursor: &mut Cursor) -> Result<ValueExpr> {
	cursor.eat("$");
	let name = cursor.take_while(|ch| ch.is_alphanumeric() || ch == '_');
	if name.is_empty() {
		bevybail!("expected an entity name after `$`");
	}
	Ok(ValueExpr::EntityRef(name.into()))
}

/// Parse a dotted field path, eg `count`, `user.name`.
fn parse_field_path(cursor: &mut Cursor) -> Result<FieldPath> {
	let mut segments = Vec::new();
	loop {
		let segment = cursor.take_while(|ch| ch.is_alphanumeric() || ch == '_');
		if segment.is_empty() {
			bevybail!("expected a field path segment");
		}
		segments.push(FieldSegment::key(segment));
		if cursor.peek() == Some('.') {
			cursor.bump();
		} else {
			break;
		}
	}
	Ok(FieldPath::new(
		segments
			.into_iter()
			.map(|segment| match segment {
				FieldSegment::ObjectKey(key) => key.to_string(),
				FieldSegment::ArrayIndex(index) => index.to_string(),
			})
			.collect::<Vec<_>>(),
	))
}

/// Parse a double-quoted string with the common escapes.
fn parse_string(cursor: &mut Cursor) -> Result<SmolStr> {
	cursor.eat("\"");
	let mut out = String::new();
	while let Some(ch) = cursor.bump() {
		match ch {
			'"' => return Ok(out.into()),
			'\\' => match cursor.bump() {
				Some('n') => out.push('\n'),
				Some('t') => out.push('\t'),
				Some('r') => out.push('\r'),
				Some('"') => out.push('"'),
				Some('\\') => out.push('\\'),
				Some(other) => out.push(other),
				None => bevybail!("unterminated string escape"),
			},
			other => out.push(other),
		}
	}
	bevybail!("unterminated string literal")
}

/// Parse a number literal as the natural [`Value`] kind by its text.
fn parse_number(cursor: &mut Cursor) -> Result<DataLiteral> {
	let text = cursor.take_while(|ch| {
		ch.is_ascii_digit()
			|| ch == '-'
			|| ch == '+'
			|| ch == '.'
			|| ch == 'e'
			|| ch == 'E'
	});
	let value = if text.contains('.') || text.contains('e') || text.contains('E') {
		Value::Float(text.parse()?)
	} else if let Ok(uint) = text.parse::<u64>() {
		Value::Uint(uint)
	} else {
		Value::Int(text.parse()?)
	};
	Ok(DataLiteral::Scalar(value))
}

/// Parse a `[a, b, c]` list literal.
fn parse_list(cursor: &mut Cursor) -> Result<DataLiteral> {
	cursor.eat("[");
	let mut items = Vec::new();
	loop {
		cursor.skip_ws();
		if cursor.eat("]") {
			break;
		}
		items.push(parse_literal(cursor)?);
		cursor.skip_ws();
		if !cursor.eat(",") && cursor.peek() != Some(']') {
			bevybail!("expected `,` or `]` in list");
		}
	}
	Ok(DataLiteral::List(items))
}

/// Parse a `{ key: value, .. }` struct literal body (braces inclusive).
fn parse_struct(cursor: &mut Cursor) -> Result<Vec<(String, DataLiteral)>> {
	cursor.eat("{");
	let mut fields = Vec::new();
	loop {
		cursor.skip_ws();
		if cursor.eat("}") {
			break;
		}
		let key = cursor.take_while(|ch| ch.is_alphanumeric() || ch == '_');
		if key.is_empty() {
			bevybail!("expected a struct field name");
		}
		cursor.skip_ws();
		if !cursor.eat(":") {
			bevybail!("expected `:` after struct field `{key}`");
		}
		let value = parse_literal(cursor)?;
		fields.push((key.to_string(), value));
		cursor.skip_ws();
		if !cursor.eat(",") && cursor.peek() != Some('}') {
			bevybail!("expected `,` or `}}` in struct");
		}
	}
	Ok(fields)
}

/// Parse an identifier-led value: `true`/`false`, or an enum/typed name with
/// optional unit/tuple/struct fields.
fn parse_ident_value(cursor: &mut Cursor) -> Result<DataLiteral> {
	let ident = cursor.take_while(|ch| {
		ch.is_alphanumeric() || ch == '_' || ch == ':'
	});
	match ident {
		"true" => return Ok(DataLiteral::Scalar(Value::Bool(true))),
		"false" => return Ok(DataLiteral::Scalar(Value::Bool(false))),
		_ => {}
	}
	let fields = parse_named_fields(cursor)?;
	Ok(DataLiteral::Enum(NamedLiteral {
		name: ident.to_string(),
		fields,
	}))
}

/// Parse a named literal `Name`, `Name(..)`, or `Name { .. }`, the form shared
/// by enum variants and spread components/templates.
fn parse_named_literal(cursor: &mut Cursor) -> Result<NamedLiteral> {
	cursor.skip_ws();
	let name = cursor
		.take_while(|ch| ch.is_alphanumeric() || ch == '_' || ch == ':');
	if name.is_empty() {
		bevybail!("expected a name in spread or enum value");
	}
	let fields = parse_named_fields(cursor)?;
	Ok(NamedLiteral {
		name: name.to_string(),
		fields,
	})
}

/// Parse the fields trailing a name: nothing (unit), `(..)` (tuple), or `{..}`
/// (struct).
fn parse_named_fields(cursor: &mut Cursor) -> Result<NamedFields> {
	match cursor.peek() {
		Some('(') => {
			cursor.bump();
			let mut items = Vec::new();
			loop {
				cursor.skip_ws();
				if cursor.eat(")") {
					break;
				}
				items.push(parse_literal(cursor)?);
				cursor.skip_ws();
				if !cursor.eat(",") && cursor.peek() != Some(')') {
					bevybail!("expected `,` or `)` in tuple value");
				}
			}
			Ok(NamedFields::Tuple(items))
		}
		Some('{') => parse_struct(cursor).map(NamedFields::Struct),
		_ => Ok(NamedFields::Unit),
	}
}

#[cfg(test)]
mod test {
	use super::*;

	fn value(input: &str) -> ValueExpr {
		parse_value_expr(&mut Cursor::new(input)).unwrap()
	}

	#[beet_core::test]
	fn scalars() {
		value("42").xpect_eq(ValueExpr::Literal(DataLiteral::Scalar(
			Value::Uint(42),
		)));
		value("-3").xpect_eq(ValueExpr::Literal(DataLiteral::Scalar(
			Value::Int(-3),
		)));
		value("2.5").xpect_eq(ValueExpr::Literal(DataLiteral::Scalar(
			Value::Float(2.5),
		)));
		value("\"hi\"").xpect_eq(ValueExpr::Literal(DataLiteral::Scalar(
			Value::Str("hi".into()),
		)));
		value("true").xpect_eq(ValueExpr::Literal(DataLiteral::Scalar(
			Value::Bool(true),
		)));
	}

	#[beet_core::test]
	fn struct_literal() {
		let ValueExpr::Literal(DataLiteral::Struct(fields)) =
			value("{x:0,y:2}")
		else {
			panic!("expected struct");
		};
		fields.len().xpect_eq(2);
		fields[0].0.clone().xpect_eq("x".to_string());
	}

	#[beet_core::test]
	fn list_literal() {
		let ValueExpr::Literal(DataLiteral::List(items)) = value("[1,2,3]")
		else {
			panic!("expected list");
		};
		items.len().xpect_eq(3);
	}

	#[beet_core::test]
	fn enum_variant() {
		let ValueExpr::Literal(DataLiteral::Enum(named)) = value("Center")
		else {
			panic!("expected enum");
		};
		named.name.xpect_eq("Center".to_string());
		named.fields.xpect_eq(NamedFields::Unit);
	}

	#[beet_core::test]
	fn field_ref_with_init() {
		let ValueExpr::FieldRef { path, init } = value("#user.name=\"x\"")
		else {
			panic!("expected field ref");
		};
		path.to_string().xpect_eq("user.name".to_string());
		init.xpect_eq(Some(DataLiteral::Scalar(Value::Str("x".into()))));
	}

	#[beet_core::test]
	fn entity_ref() {
		value("$header")
			.xpect_eq(ValueExpr::EntityRef("header".into()));
	}

	#[beet_core::test]
	fn spread_named_and_tuple() {
		let SpreadExpr::Named(named) =
			parse_spread(&mut Cursor::new("MyComponent{foo:\"bar\"}")).unwrap()
		else {
			panic!("expected named spread");
		};
		named.name.xpect_eq("MyComponent".to_string());

		let SpreadExpr::Tuple(items) =
			parse_spread(&mut Cursor::new("(A, B)")).unwrap()
		else {
			panic!("expected tuple spread");
		};
		items.len().xpect_eq(2);
	}
}
