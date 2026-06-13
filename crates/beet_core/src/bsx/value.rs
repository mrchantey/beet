//! The BSX value grammar: literals, `@` bindings, `$` references, and spreads.
//!
//! A hand-written cursor over the value surface, mirroring the `bsn!` surface:
//! scalars, named-field structs, lists, and enums (unit/tuple/struct variants),
//! plus `@source:path` reactive bindings ([`BindingExpr`]) and `$entity`
//! references. Position decides which entry point is called:
//! [`parse_value_expr`] for attribute-value and text position, [`parse_spread`]
//! for bare-child position. Both share the literal grammar ([`parse_literal`]).

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
		Some('#') => bevybail!(
			"the `#field` syntax was removed, use `@doc:field` (init form `@doc:field=0`)"
		),
		Some('$') => parse_entity_ref(cursor),
		Some('@') => parse_binding(cursor).map(ValueExpr::Binding),
		_ => parse_literal(cursor).map(ValueExpr::Literal),
	}
}

/// Parse a bare-position spread `{MyComponent{..}}` or `{(A, B)}`. A tuple item
/// may also be an `@` binding, eg `{(Bar{boo:"bazz"}, @comp:Bar.boo)}`.
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
			items.push(match cursor.peek() {
				Some('@') => parse_binding(cursor).map(SpreadItem::Binding)?,
				_ => parse_named_literal(cursor).map(SpreadItem::Named)?,
			});
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

/// Parse an `@source selector? : path init?` binding, the module-level grammar.
/// A `$ref` selector is a `bx:ref` name or one of the reserved well-known
/// names (`BuildRoot`, `SnippetRoot`, `RenderRoot`, `Router`); reservation is
/// the resolver's concern ([`ReservedRef`](super::resolve::ReservedRef)), the
/// grammar does not distinguish them.
pub fn parse_binding(cursor: &mut Cursor) -> Result<BindingExpr> {
	cursor.eat("@");
	let source_name = cursor.take_while(|ch| ch.is_alphanumeric());
	let source = match source_name {
		"doc" => BindingSource::Doc,
		"res" => BindingSource::Res,
		"comp" => BindingSource::Comp,
		"prop" => BindingSource::Prop,
		other => bevybail!(
			"unknown binding source `@{other}`, expected `doc`, `res`, `comp` or `prop`"
		),
	};
	// an optional `$ref` selector, `@comp` only.
	let selector = if cursor.peek() == Some('$') {
		if source != BindingSource::Comp {
			bevybail!(
				"a `$ref` selector is only valid on `@comp`, found `@{source_name}$`"
			);
		}
		cursor.bump();
		let name = cursor.take_while(|ch| ch.is_alphanumeric() || ch == '_');
		if name.is_empty() {
			bevybail!("expected a ref name after `@comp$`");
		}
		Some(SmolStr::from(name))
	} else {
		None
	};
	if !cursor.eat(":") {
		bevybail!("expected `:` after the `@{source_name}` binding source");
	}
	// `@res`/`@comp` lead with a `ShortTypePath.` segment.
	let type_path = match source {
		BindingSource::Res | BindingSource::Comp => {
			let type_path = cursor
				.take_while(|ch| ch.is_alphanumeric() || ch == '_' || ch == ':');
			if type_path.is_empty() {
				bevybail!("`@{source_name}:` expects a `Type.field` path");
			}
			if !cursor.eat(".") {
				bevybail!(
					"`@{source_name}:{type_path}` is missing its field path, expected `@{source_name}:{type_path}.field`"
				);
			}
			Some(SmolStr::from(type_path))
		}
		BindingSource::Doc | BindingSource::Prop => None,
	};
	let field_path = parse_field_path(cursor)?;
	// an optional `=literal` initializer, `@doc` only.
	let init = if cursor.peek() == Some('=') {
		if source != BindingSource::Doc {
			bevybail!("an `=init` is only valid on `@doc` bindings");
		}
		cursor.bump();
		Some(parse_literal(cursor)?)
	} else {
		None
	};
	BindingExpr {
		source,
		selector,
		type_path,
		field_path,
		init,
	}
	.xok()
}

/// Parse a `verb{ arg: value, .. }` event-verb call, the value of a `bx:<event>`
/// directive. The verb is a bare identifier; the brace map is optional (a verb
/// may take no arguments, eg `submit`). Each argument value shares the
/// attribute-value grammar ([`parse_value_expr`]), so a literal and an `@`
/// binding both parse, the latter kept as a [`VerbArg::Binding`].
pub fn parse_verb_call(cursor: &mut Cursor) -> Result<VerbCall> {
	cursor.skip_ws();
	let verb = cursor.take_while(|ch| ch.is_alphanumeric() || ch == '_');
	if verb.is_empty() {
		bevybail!("expected a verb name, ie `increment{{ field: @doc:count }}`");
	}
	cursor.skip_ws();
	let args = match cursor.peek() {
		Some('{') => parse_verb_args(cursor)?,
		_ => Vec::new(),
	};
	Ok(VerbCall {
		verb: verb.into(),
		args,
	})
}

/// Parse the `{ key: value, .. }` argument map of a verb call (braces inclusive),
/// each value a literal or an `@` binding.
fn parse_verb_args(cursor: &mut Cursor) -> Result<Vec<(SmolStr, VerbArg)>> {
	cursor.eat("{");
	let mut args = Vec::new();
	loop {
		cursor.skip_ws();
		if cursor.eat("}") {
			break;
		}
		let key = cursor.take_while(|ch| ch.is_alphanumeric() || ch == '_');
		if key.is_empty() {
			bevybail!("expected a verb argument name");
		}
		cursor.skip_ws();
		if !cursor.eat(":") {
			bevybail!("expected `:` after verb argument `{key}`");
		}
		let arg = match parse_value_expr(cursor)? {
			ValueExpr::Binding(binding) => VerbArg::Binding(binding),
			ValueExpr::Literal(literal) => VerbArg::Literal(literal),
			ValueExpr::EntityRef(_) => bevybail!(
				"a `$name` entity reference is not a verb argument value"
			),
		};
		args.push((key.into(), arg));
		cursor.skip_ws();
		if !cursor.eat(",") && cursor.peek() != Some('}') {
			bevybail!("expected `,` or `}}` in verb arguments");
		}
	}
	Ok(args)
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
fn parse_struct(cursor: &mut Cursor) -> Result<Vec<(SmolStr, DataLiteral)>> {
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
		fields.push((key.into(), value));
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
		name: ident.into(),
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
		name: name.into(),
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
		fields[0].0.as_str().xpect_eq("x");
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
		named.name.as_str().xpect_eq("Center");
		named.fields.xpect_eq(NamedFields::Unit);
	}

	#[beet_core::test]
	fn removed_field_ref_syntax_errors() {
		parse_err("#count").xpect_contains("use `@doc:field`");
		parse_err("#user.name=\"x\"").xpect_contains("removed");
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
		named.name.as_str().xpect_eq("MyComponent");

		let SpreadExpr::Tuple(items) =
			parse_spread(&mut Cursor::new("(A, B)")).unwrap()
		else {
			panic!("expected tuple spread");
		};
		items.len().xpect_eq(2);
	}

	/// Shorthand for the parsed [`BindingExpr`] of a binding source string.
	fn binding(input: &str) -> BindingExpr {
		let ValueExpr::Binding(binding) = value(input) else {
			panic!("expected binding for `{input}`");
		};
		binding
	}

	#[beet_core::test]
	fn doc_binding() {
		let parsed = binding("@doc:count");
		parsed.source.xpect_eq(BindingSource::Doc);
		parsed.selector.xpect_eq(None);
		parsed.type_path.xpect_eq(None);
		parsed.field_path.to_string().xpect_eq("count".to_string());
		parsed.init.xpect_eq(None);
	}

	#[beet_core::test]
	fn doc_binding_with_init() {
		let parsed = binding("@doc:user.name=\"x\"");
		parsed.field_path.to_string().xpect_eq("user.name".to_string());
		parsed
			.init
			.xpect_eq(Some(DataLiteral::Scalar(Value::Str("x".into()))));
	}

	#[beet_core::test]
	fn res_binding() {
		let parsed = binding("@res:PackageConfig.title");
		parsed.source.xpect_eq(BindingSource::Res);
		parsed.type_path.xpect_eq(Some("PackageConfig".into()));
		parsed.field_path.to_string().xpect_eq("title".to_string());
	}

	#[beet_core::test]
	fn comp_binding_with_selector() {
		let parsed = binding("@comp$myref:Bar.boo");
		parsed.source.xpect_eq(BindingSource::Comp);
		parsed.selector.xpect_eq(Some("myref".into()));
		parsed.type_path.xpect_eq(Some("Bar".into()));
		parsed.field_path.to_string().xpect_eq("boo".to_string());
	}

	#[beet_core::test]
	fn comp_binding_nested_field() {
		let parsed = binding("@comp:Bar.style.width");
		parsed.type_path.xpect_eq(Some("Bar".into()));
		parsed
			.field_path
			.to_string()
			.xpect_eq("style.width".to_string());
	}

	#[beet_core::test]
	fn prop_binding() {
		let parsed = binding("@prop:title");
		parsed.source.xpect_eq(BindingSource::Prop);
		parsed.type_path.xpect_eq(None);
		parsed.field_path.to_string().xpect_eq("title".to_string());
	}

	/// Shorthand for the parse error of a value source string.
	fn parse_err(input: &str) -> String {
		parse_value_expr(&mut Cursor::new(input))
			.unwrap_err()
			.to_string()
	}

	#[beet_core::test]
	fn binding_errors() {
		parse_err("@bogus:x").xpect_contains("unknown binding source");
		parse_err("@doc count").xpect_contains("expected `:`");
		parse_err("@res:NoField").xpect_contains("missing its field path");
		parse_err("@comp:").xpect_contains("expects a `Type.field` path");
		parse_err("@prop$x:title")
			.xpect_contains("only valid on `@comp`");
		parse_err("@res:Type.field=1")
			.xpect_contains("only valid on `@doc`");
		parse_err("@comp$:Bar.boo")
			.xpect_contains("expected a ref name");
	}

	#[beet_core::test]
	fn spread_tuple_with_binding() {
		let SpreadExpr::Tuple(items) =
			parse_spread(&mut Cursor::new("(Bar{boo:\"bazz\"}, @comp:Bar.boo)"))
				.unwrap()
		else {
			panic!("expected tuple spread");
		};
		items.len().xpect_eq(2);
		let SpreadItem::Named(named) = &items[0] else {
			panic!("expected named item");
		};
		named.name.as_str().xpect_eq("Bar");
		let SpreadItem::Binding(parsed) = &items[1] else {
			panic!("expected binding item");
		};
		parsed.source.xpect_eq(BindingSource::Comp);
		parsed.type_path.clone().xpect_eq(Some("Bar".into()));
	}

	/// Shorthand for a parsed verb call.
	fn verb(input: &str) -> VerbCall {
		parse_verb_call(&mut Cursor::new(input)).unwrap()
	}

	#[beet_core::test]
	fn verb_call_literal_and_binding_args() {
		let call = verb("increment{ field: @doc:count, amount: 3 }");
		call.verb.as_str().xpect_eq("increment");
		call.args.len().xpect_eq(2);
		// `field` is a binding, `amount` a literal.
		let (field_name, field_arg) = &call.args[0];
		field_name.as_str().xpect_eq("field");
		let VerbArg::Binding(binding) = field_arg else {
			panic!("expected a binding arg");
		};
		binding.source.xpect_eq(BindingSource::Doc);
		binding.field_path.to_string().xpect_eq("count".to_string());
		let (amount_name, amount_arg) = &call.args[1];
		amount_name.as_str().xpect_eq("amount");
		amount_arg
			.clone()
			.xpect_eq(VerbArg::Literal(DataLiteral::Scalar(Value::Uint(3))));
	}

	#[beet_core::test]
	fn verb_call_no_args() {
		// a bare verb name with no brace map is a zero-argument verb.
		let call = verb("submit");
		call.verb.as_str().xpect_eq("submit");
		call.args.is_empty().xpect_true();
	}

	#[beet_core::test]
	fn verb_call_doc_init_arg() {
		// a binding arg keeps its `=init`, eg `@doc:count=0`.
		let call = verb("increment{ field: @doc:count=0 }");
		let VerbArg::Binding(binding) = &call.args[0].1 else {
			panic!("expected a binding arg");
		};
		binding
			.init
			.clone()
			.xpect_eq(Some(DataLiteral::Scalar(Value::Uint(0))));
	}

	/// Shorthand for the parse error of a verb-call source string.
	fn verb_err(input: &str) -> String {
		parse_verb_call(&mut Cursor::new(input))
			.unwrap_err()
			.to_string()
	}

	#[beet_core::test]
	fn verb_call_errors() {
		verb_err("{ field: @doc:count }").xpect_contains("expected a verb name");
		verb_err("increment{ : @doc:count }")
			.xpect_contains("expected a verb argument name");
		verb_err("increment{ field @doc:count }")
			.xpect_contains("expected `:` after verb argument");
		verb_err("increment{ field: @doc:count amount: 3 }")
			.xpect_contains("expected `,` or `}`");
	}
}
