//! Frontmatter parsing for YAML and TOML metadata blocks.
//!
//! Provides the [`Frontmatter`] component and lightweight hand-rolled
//! parsers for simple key-value frontmatter. Values are parsed into
//! the existing [`Value`](crate::prelude::Value) type and assembled
//! into a `bevy::reflect::DynamicStruct`.

use beet_core::prelude::*;
use bevy::reflect::DynamicStruct;

/// Parsed frontmatter metadata from a YAML or TOML block.
///
/// Inserted on the root entity when frontmatter is present in the
/// markdown source. The `value` field is a [`DynamicStruct`] built
/// from the parsed key-value pairs, suitable for reflection-based
/// access.
///
/// ## Example
/// ```rust
/// # use beet_node::prelude::*;
/// # use beet_core::prelude::*;
/// let fm = Frontmatter::parse("title: Hello\nauthor: World", FrontmatterKind::Yaml).unwrap();
/// fm.kind.xpect_eq(FrontmatterKind::Yaml);
/// ```
#[derive(Debug, Component)]
pub struct Frontmatter {
	/// The parsed metadata as a dynamic struct for reflection.
	pub value: DynamicStruct,
	/// The frontmatter format that was parsed.
	pub kind: FrontmatterKind,
}

/// The format of a frontmatter metadata block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrontmatterKind {
	/// YAML frontmatter delimited by `---`.
	Yaml,
	/// TOML frontmatter delimited by `+++`.
	Toml,
}

impl Frontmatter {
	/// Parse a raw frontmatter string into a [`Frontmatter`] component.
	///
	/// Dispatches to the appropriate parser based on `kind`.
	pub fn parse(content: &str, kind: FrontmatterKind) -> Result<Self> {
		let pairs = match kind {
			FrontmatterKind::Yaml => parse_yaml_kv(content)?,
			FrontmatterKind::Toml => parse_toml_kv(content)?,
		};
		let value = build_dynamic_struct(pairs)?;
		Ok(Self { value, kind })
	}

	/// Get a string field from the frontmatter by name.
	///
	/// Returns `None` if the field does not exist or is not a string.
	pub fn get_str(&self, key: &str) -> Option<&str> {
		self.value
			.field(key)
			.and_then(|field| field.try_downcast_ref::<String>())
			.map(|s| s.as_str())
	}
}

/// Build a [`DynamicStruct`] from a list of key-value pairs.
fn build_dynamic_struct(pairs: Vec<(String, Value)>) -> Result<DynamicStruct> {
	let mut dynamic = DynamicStruct::default();
	for (key, value) in pairs {
		match value {
			Value::Null => {
				dynamic.insert(&key, ());
			}
			Value::Bool(val) => {
				dynamic.insert(&key, val);
			}
			Value::Int(val) => {
				dynamic.insert(&key, val);
			}
			Value::Uint(val) => {
				dynamic.insert(&key, val);
			}
			Value::Float(val) => {
				dynamic.insert(&key, val);
			}
			Value::Str(val) => {
				dynamic.insert(&key, val);
			}
			Value::Bytes(val) => {
				dynamic.insert(&key, val);
			}
			Value::Map(_) | Value::List(_) => {
				bevybail!(
					"Unsupported complex value for frontmatter key '{}'",
					key
				);
			}
		}
	}
	dynamic.xok()
}

/// Parse simple YAML key-value pairs.
///
/// Supports flat `key: value` lines with scalar values. Blank lines
/// and comment lines (starting with `#`) are skipped. Quoted string
/// values (single or double) have their quotes stripped.
fn parse_yaml_kv(content: &str) -> Result<Vec<(String, Value)>> {
	let mut pairs = Vec::new();

	for line in content.lines() {
		let trimmed = line.trim();

		// skip blanks and comments
		if trimmed.is_empty() || trimmed.starts_with('#') {
			continue;
		}

		// find the colon separator
		let Some(colon_pos) = trimmed.find(':') else {
			continue;
		};

		let key = trimmed[..colon_pos].trim().to_string();
		if key.is_empty() {
			continue;
		}

		let raw_value = trimmed[colon_pos + 1..].trim();
		let value = parse_yaml_value(raw_value);
		pairs.push((key, value));
	}

	Ok(pairs)
}

/// Parse a single YAML scalar value string into a [`Value`].
fn parse_yaml_value(raw: &str) -> Value {
	if raw.is_empty() || raw == "~" || raw == "null" || raw == "Null" {
		return Value::Null;
	}

	// strip inline comments (but not inside quotes)
	let effective = if !raw.starts_with('"')
		&& !raw.starts_with('\'')
		&& let Some(comment_pos) = raw.find(" #")
	{
		raw[..comment_pos].trim()
	} else {
		raw
	};

	// strip quotes
	let unquoted = strip_quotes(effective);

	// if it was quoted, treat as string
	if unquoted.len() != effective.len() {
		return Value::Str(unquoted.to_string());
	}

	// try parsing as typed value
	Value::parse_string(unquoted)
}

/// Parse simple TOML key-value pairs.
///
/// Supports flat `key = value` lines. Blank lines and comment lines
/// (starting with `#`) are skipped. Section headers (`[section]`) are
/// currently ignored. String values must be quoted.
fn parse_toml_kv(content: &str) -> Result<Vec<(String, Value)>> {
	let mut pairs = Vec::new();

	for line in content.lines() {
		let trimmed = line.trim();

		// skip blanks, comments, and section headers
		if trimmed.is_empty()
			|| trimmed.starts_with('#')
			|| trimmed.starts_with('[')
		{
			continue;
		}

		// find the equals separator
		let Some(eq_pos) = trimmed.find('=') else {
			continue;
		};

		let key = trimmed[..eq_pos].trim().to_string();
		if key.is_empty() {
			continue;
		}

		let raw_value = trimmed[eq_pos + 1..].trim();
		let value = parse_toml_value(raw_value);
		pairs.push((key, value));
	}

	Ok(pairs)
}

/// Parse a single TOML value string into a [`Value`].
fn parse_toml_value(raw: &str) -> Value {
	if raw.is_empty() {
		return Value::Null;
	}

	// strip inline comments (but not inside quotes)
	let effective = if !raw.starts_with('"')
		&& !raw.starts_with('\'')
		&& let Some(comment_pos) = raw.find(" #")
	{
		raw[..comment_pos].trim()
	} else {
		raw
	};

	// TOML booleans
	if effective == "true" {
		return Value::Bool(true);
	}
	if effective == "false" {
		return Value::Bool(false);
	}

	// quoted strings
	let unquoted = strip_quotes(effective);
	if unquoted.len() != effective.len() {
		return Value::Str(unquoted.to_string());
	}

	// try numeric parsing
	Value::parse_string(effective)
}

/// Strip matching single or double quotes from a string.
fn strip_quotes(val: &str) -> &str {
	if val.len() >= 2 {
		if (val.starts_with('"') && val.ends_with('"'))
			|| (val.starts_with('\'') && val.ends_with('\''))
		{
			return &val[1..val.len() - 1];
		}
	}
	val
}


#[cfg(test)]
mod test {
	use super::*;

	// -- YAML parsing --

	#[test]
	fn yaml_simple_string() {
		let pairs = parse_yaml_kv("title: Hello World").unwrap();
		pairs.len().xpect_eq(1);
		pairs[0].0.as_str().xpect_eq("title");
		pairs[0].1.to_string().xpect_eq("Hello World");
	}

	#[test]
	fn yaml_quoted_string() {
		let pairs = parse_yaml_kv("title: \"Hello World\"").unwrap();
		pairs[0].1.xpect_eq(Value::Str("Hello World".into()));
	}

	#[test]
	fn yaml_single_quoted_string() {
		let pairs = parse_yaml_kv("title: 'Hello World'").unwrap();
		pairs[0].1.xpect_eq(Value::Str("Hello World".into()));
	}

	#[test]
	fn yaml_boolean() {
		let pairs = parse_yaml_kv("draft: true\npublished: false").unwrap();
		pairs[0].1.xpect_eq(Value::Bool(true));
		pairs[1].1.xpect_eq(Value::Bool(false));
	}

	#[test]
	fn yaml_integer() {
		let pairs = parse_yaml_kv("count: 42").unwrap();
		pairs[0].1.xpect_eq(Value::Uint(42));
	}

	#[test]
	fn yaml_negative_integer() {
		let pairs = parse_yaml_kv("offset: -7").unwrap();
		pairs[0].1.xpect_eq(Value::Int(-7));
	}

	#[test]
	fn yaml_float() {
		let pairs = parse_yaml_kv("weight: 3.14").unwrap();
		pairs[0].1.xpect_eq(Value::Float(3.14));
	}

	#[test]
	fn yaml_null_variants() {
		for input in ["empty:", "tilde: ~", "null_word: null"] {
			let pairs = parse_yaml_kv(input).unwrap();
			pairs[0].1.xpect_eq(Value::Null);
		}
	}

	#[test]
	fn yaml_skips_comments_and_blanks() {
		let content = "# comment\n\ntitle: Hello\n# another\nauthor: World";
		let pairs = parse_yaml_kv(content).unwrap();
		pairs.len().xpect_eq(2);
		pairs[0].0.as_str().xpect_eq("title");
		pairs[1].0.as_str().xpect_eq("author");
	}

	#[test]
	fn yaml_inline_comment() {
		let pairs = parse_yaml_kv("title: Hello # a comment").unwrap();
		pairs[0].1.to_string().xpect_eq("Hello");
	}

	#[test]
	fn yaml_multiple_pairs() {
		let content = "title: My Post\nauthor: Jane\ntags: rust, bevy";
		let pairs = parse_yaml_kv(content).unwrap();
		pairs.len().xpect_eq(3);
	}

	// -- TOML parsing --

	#[test]
	fn toml_quoted_string() {
		let pairs = parse_toml_kv("title = \"Hello World\"").unwrap();
		pairs.len().xpect_eq(1);
		pairs[0].0.as_str().xpect_eq("title");
		pairs[0].1.xpect_eq(Value::Str("Hello World".into()));
	}

	#[test]
	fn toml_boolean() {
		let pairs = parse_toml_kv("draft = true").unwrap();
		pairs[0].1.xpect_eq(Value::Bool(true));
	}

	#[test]
	fn toml_integer() {
		let pairs = parse_toml_kv("count = 42").unwrap();
		pairs[0].1.xpect_eq(Value::Uint(42));
	}

	#[test]
	fn toml_float() {
		let pairs = parse_toml_kv("weight = 3.14").unwrap();
		pairs[0].1.xpect_eq(Value::Float(3.14));
	}

	#[test]
	fn toml_skips_sections() {
		let content = "[meta]\ntitle = \"Hello\"\n[other]\ncount = 5";
		let pairs = parse_toml_kv(content).unwrap();
		pairs.len().xpect_eq(2);
	}

	#[test]
	fn toml_skips_comments() {
		let content = "# comment\ntitle = \"Hello\"";
		let pairs = parse_toml_kv(content).unwrap();
		pairs.len().xpect_eq(1);
	}

	// -- Frontmatter component --

	#[test]
	fn frontmatter_yaml() {
		let fm = Frontmatter::parse(
			"title: Hello\nauthor: World",
			FrontmatterKind::Yaml,
		)
		.unwrap();
		fm.kind.xpect_eq(FrontmatterKind::Yaml);
		fm.value.field_len().xpect_eq(2);
	}

	#[test]
	fn frontmatter_toml() {
		let fm = Frontmatter::parse(
			"title = \"Hello\"\ncount = 42",
			FrontmatterKind::Toml,
		)
		.unwrap();
		fm.kind.xpect_eq(FrontmatterKind::Toml);
		fm.value.field_len().xpect_eq(2);
	}

	#[test]
	fn frontmatter_empty() {
		let fm = Frontmatter::parse("", FrontmatterKind::Yaml).unwrap();
		fm.value.field_len().xpect_eq(0);
	}

	#[test]
	fn get_str_field() {
		let fm = Frontmatter::parse(
			"title: Hello\ncount: 42",
			FrontmatterKind::Yaml,
		)
		.unwrap();
		fm.get_str("title").unwrap().xpect_eq("Hello");
		fm.get_str("count").is_none().xpect_true();
		fm.get_str("missing").is_none().xpect_true();
	}

	#[test]
	fn dynamic_struct_fields_accessible() {
		let fm = Frontmatter::parse(
			"title: Hello\ncount: 42\ndraft: true",
			FrontmatterKind::Yaml,
		)
		.unwrap();
		fm.value.field_len().xpect_eq(3);
		fm.value.field("title").is_some().xpect_true();
		fm.value.field("count").is_some().xpect_true();
		fm.value.field("draft").is_some().xpect_true();
	}
}
