//! Frontmatter parsing for YAML and TOML metadata blocks.
//!
//! Provides the [`Frontmatter`] component and lightweight hand-rolled parsers
//! for simple key-value frontmatter. Values parse into the existing
//! [`Value`](beet_core::prelude::Value) type, and the block as a whole lowers to
//! [`RootDeclarations`]: frontmatter is the markdown surface for a document's
//! root component declarations exactly as a root spread is the BSX surface, so
//! both resolve through one reflect path rather than a hand-written key mapping.

use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::structs::DynamicStruct;

/// Parsed frontmatter metadata from a YAML or TOML block.
///
/// Inserted on the root entity when frontmatter is present in the markdown
/// source. The `value` field is a [`DynamicStruct`] built from the unsectioned
/// key-value pairs, for reflection-based access by a document's own systems; the
/// route-metadata path instead reads [`declarations`](Self::declarations).
///
/// ## Example
/// ```rust
/// # use beet_ui::prelude::*;
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
	/// The parsed pairs grouped by the component their section names, in source
	/// order. Unsectioned pairs (and all of YAML, which is flat) carry `None` and
	/// target the document's [`FrontmatterType`].
	sections: Vec<FrontmatterSection>,
}

/// The component a document's unsectioned frontmatter keys declare, resolved by
/// self-or-ancestor from whichever entity discovered the document.
///
/// Defaults to [`PageMeta`], the general page metadata beet ships. A site
/// declaring its own names it on the dir that discovers those documents,
/// `<RoutesDir src="routes" {FrontmatterType{component:"MyMeta"}}/>`, and a TOML
/// `[Section]` header names a component explicitly, winning over this default.
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct FrontmatterType {
	/// Short type path of the component, resolved against the type registry
	/// exactly as a BSX tag is.
	pub component: SmolStr,
}

impl Default for FrontmatterType {
	fn default() -> Self {
		Self {
			component: type_ext::short_name::<PageMeta>(),
		}
	}
}

/// One `[Component]` group of a frontmatter block.
#[derive(Debug)]
struct FrontmatterSection {
	/// The component this group's keys declare, `None` for the unsectioned
	/// group targeting the document's [`FrontmatterType`].
	component: Option<SmolStr>,
	pairs: Vec<(String, Value)>,
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
	/// Extract and parse a leading frontmatter block from a raw source: a
	/// `---` fence parses as YAML, a `+++` fence as TOML. Returns `None` when
	/// the source has no leading fence.
	///
	/// The full markdown parser extracts frontmatter as part of its pass; this
	/// standalone form serves scan-time metadata readers (eg route discovery)
	/// that must not pay for a content parse.
	pub fn extract(source: &str) -> Result<Option<Self>> {
		let trimmed = source.trim_start();
		let (fence, kind) = match trimmed {
			s if s.starts_with("---") => ("---", FrontmatterKind::Yaml),
			s if s.starts_with("+++") => ("+++", FrontmatterKind::Toml),
			_ => return Ok(None),
		};
		// the fence must be a whole line, ie not a thematic break `--- foo`
		let body = &trimmed[fence.len()..];
		if !body.starts_with('\n') && !body.starts_with("\r\n") {
			return Ok(None);
		}
		let Some(end) = body.find(&format!("\n{fence}")) else {
			return Ok(None);
		};
		Self::parse(&body[..end], kind).map(Some)
	}

	/// Parse a raw frontmatter string into a [`Frontmatter`] component.
	///
	/// Dispatches to the appropriate parser based on `kind`.
	pub fn parse(content: &str, kind: FrontmatterKind) -> Result<Self> {
		let sections = match kind {
			FrontmatterKind::Yaml => vec![FrontmatterSection {
				component: None,
				pairs: parse_yaml_kv(content)?,
			}],
			FrontmatterKind::Toml => parse_toml_sections(content)?,
		};
		// the reflect surface mirrors the default component's keys, ie the
		// unsectioned ones; a `[Section]` group is read through `declarations`.
		let value = sections
			.iter()
			.filter(|section| section.component.is_none())
			.flat_map(|section| section.pairs.iter().cloned())
			.collect::<Vec<_>>()
			.xmap(build_dynamic_struct)?;
		Ok(Self {
			value,
			kind,
			sections,
		})
	}

	/// This block as document root declarations: each `[Section]` names its own
	/// component by short type path, and every unsectioned key declares
	/// `default_component` (the document's [`FrontmatterType`]).
	///
	/// The one lowering, so a frontmatter key coerces to its field type through
	/// the same [`DataLiteral`] rules a BSX spread does — `created = "2025-07-11"`
	/// becomes a [`Timestamp`], `order = 2` a `u32`. A key with no value declares
	/// nothing and is dropped rather than resolving to null.
	pub fn declarations(&self, default_component: &str) -> RootDeclarations {
		self.sections
			.iter()
			.filter_map(|section| {
				let fields = section
					.pairs
					.iter()
					.filter(|(_, value)| !matches!(value, Value::Null))
					.map(|(key, value)| {
						(SmolStr::new(key), DataLiteral::Scalar(value.clone()))
					})
					.collect::<Vec<_>>();
				(!fields.is_empty()).then(|| NamedLiteral {
					name: section
						.component
						.clone()
						.unwrap_or_else(|| SmolStr::new(default_component)),
					fields: NamedFields::Struct(fields),
				})
			})
			.collect::<Vec<_>>()
			.xmap(RootDeclarations)
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

	/// Get a boolean field from the frontmatter by name.
	///
	/// Returns `None` if the field does not exist or is not a bool.
	pub fn get_bool(&self, key: &str) -> Option<bool> {
		self.value
			.field(key)
			.and_then(|field| field.try_downcast_ref::<bool>())
			.copied()
	}

	/// Get an unsigned-integer field from the frontmatter by name.
	///
	/// Unsigned scalars parse to [`Value::Uint`](beet_core::prelude::Value::Uint)
	/// (stored as `u64`); returns `None` if the field is missing or non-uint.
	pub fn get_uint(&self, key: &str) -> Option<u64> {
		self.value
			.field(key)
			.and_then(|field| field.try_downcast_ref::<u64>())
			.copied()
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
				dynamic.insert(&key, val.to_string());
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
		return Value::str(unquoted);
	}

	// try parsing as typed value
	Value::parse_string(unquoted)
}

/// Parse TOML key-value pairs, grouped by `[Section]` header.
///
/// Supports flat `key = value` lines. Blank lines and comment lines (starting
/// with `#`) are skipped; string values must be quoted. A `[Section]` header
/// opens a new group naming the component its keys declare, so the keys before
/// the first header are the unsectioned group.
fn parse_toml_sections(content: &str) -> Result<Vec<FrontmatterSection>> {
	let mut sections = vec![FrontmatterSection {
		component: None,
		pairs: Vec::new(),
	}];

	for line in content.lines() {
		let trimmed = line.trim();

		// skip blanks and comments
		if trimmed.is_empty() || trimmed.starts_with('#') {
			continue;
		}

		// a section header opens a new group, naming its component
		if let Some(header) = trimmed.strip_prefix('[')
			&& let Some(name) = header.strip_suffix(']')
		{
			sections.push(FrontmatterSection {
				component: Some(SmolStr::new(name.trim())),
				pairs: Vec::new(),
			});
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
		// unreachable-empty: the vec is seeded with the unsectioned group
		if let Some(section) = sections.last_mut() {
			section.pairs.push((key, value));
		}
	}

	Ok(sections)
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
		return Value::str(unquoted);
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

	#[beet_core::test]
	fn yaml_simple_string() {
		let pairs = parse_yaml_kv("title: Hello World").unwrap();
		pairs.len().xpect_eq(1);
		pairs[0].0.as_str().xpect_eq("title");
		pairs[0].1.to_string().xpect_eq("Hello World");
	}

	#[beet_core::test]
	fn yaml_quoted_string() {
		let pairs = parse_yaml_kv("title: \"Hello World\"").unwrap();
		pairs[0].1.xpect_eq(Value::Str("Hello World".into()));
	}

	#[beet_core::test]
	fn yaml_single_quoted_string() {
		let pairs = parse_yaml_kv("title: 'Hello World'").unwrap();
		pairs[0].1.xpect_eq(Value::Str("Hello World".into()));
	}

	#[beet_core::test]
	fn yaml_boolean() {
		let pairs = parse_yaml_kv("draft: true\npublished: false").unwrap();
		pairs[0].1.xpect_eq(Value::Bool(true));
		pairs[1].1.xpect_eq(Value::Bool(false));
	}

	#[beet_core::test]
	fn yaml_integer() {
		let pairs = parse_yaml_kv("count: 42").unwrap();
		pairs[0].1.xpect_eq(Value::Uint(42));
	}

	#[beet_core::test]
	fn yaml_negative_integer() {
		let pairs = parse_yaml_kv("offset: -7").unwrap();
		pairs[0].1.xpect_eq(Value::Int(-7));
	}

	#[beet_core::test]
	fn yaml_float() {
		let pairs = parse_yaml_kv("weight: 3.14").unwrap();
		pairs[0].1.xpect_eq(Value::Float(3.14));
	}

	#[beet_core::test]
	fn yaml_null_variants() {
		for input in ["empty:", "tilde: ~", "null_word: null"] {
			let pairs = parse_yaml_kv(input).unwrap();
			pairs[0].1.xpect_eq(Value::Null);
		}
	}

	#[beet_core::test]
	fn yaml_skips_comments_and_blanks() {
		let content = "# comment\n\ntitle: Hello\n# another\nauthor: World";
		let pairs = parse_yaml_kv(content).unwrap();
		pairs.len().xpect_eq(2);
		pairs[0].0.as_str().xpect_eq("title");
		pairs[1].0.as_str().xpect_eq("author");
	}

	#[beet_core::test]
	fn yaml_inline_comment() {
		let pairs = parse_yaml_kv("title: Hello # a comment").unwrap();
		pairs[0].1.to_string().xpect_eq("Hello");
	}

	#[beet_core::test]
	fn yaml_multiple_pairs() {
		let content = "title: My Post\nauthor: Jane\ntags: rust, bevy";
		let pairs = parse_yaml_kv(content).unwrap();
		pairs.len().xpect_eq(3);
	}

	// -- TOML parsing --

	/// The unsectioned pairs, ie the group targeting the default component.
	fn toml_pairs(content: &str) -> Vec<(String, Value)> {
		parse_toml_sections(content)
			.unwrap()
			.into_iter()
			.filter(|section| section.component.is_none())
			.flat_map(|section| section.pairs)
			.collect()
	}

	#[beet_core::test]
	fn toml_quoted_string() {
		let pairs = toml_pairs("title = \"Hello World\"");
		pairs.len().xpect_eq(1);
		pairs[0].0.as_str().xpect_eq("title");
		pairs[0].1.xpect_eq(Value::Str("Hello World".into()));
	}

	#[beet_core::test]
	fn toml_boolean() {
		toml_pairs("draft = true")[0].1.xpect_eq(Value::Bool(true));
	}

	#[beet_core::test]
	fn toml_integer() {
		toml_pairs("count = 42")[0].1.xpect_eq(Value::Uint(42));
	}

	#[beet_core::test]
	fn toml_float() {
		toml_pairs("weight = 3.14")[0]
			.1
			.xpect_eq(Value::Float(3.14));
	}

	/// A `[Section]` header names the component its keys declare, so the pairs
	/// after it leave the unsectioned group.
	#[beet_core::test]
	fn toml_groups_by_section() {
		let sections =
			parse_toml_sections("title = \"Hello\"\n[Extra]\ncount = 5")
				.unwrap();
		sections.len().xpect_eq(2);
		sections[0].component.is_none().xpect_true();
		sections[0].pairs.len().xpect_eq(1);
		sections[1].component.as_deref().unwrap().xpect_eq("Extra");
		sections[1].pairs[0].1.xpect_eq(Value::Uint(5));
	}

	#[beet_core::test]
	fn toml_skips_comments() {
		toml_pairs("# comment\ntitle = \"Hello\"").len().xpect_eq(1);
	}

	// -- root declarations --

	/// Every key lowers to a literal against the default component, so nothing
	/// hand-maps frontmatter to a struct.
	#[beet_core::test]
	fn declares_the_default_component() {
		let fm = Frontmatter::parse(
			"title = \"Hello\"\norder = 2\nempty =",
			FrontmatterKind::Toml,
		)
		.unwrap();
		let RootDeclarations(declarations) = fm.declarations("PageMeta");
		declarations.len().xpect_eq(1);
		declarations[0].name.as_str().xpect_eq("PageMeta");
		let NamedFields::Struct(fields) = &declarations[0].fields else {
			panic!("expected named fields")
		};
		// the valueless key declares nothing
		fields.len().xpect_eq(2);
		fields[0].0.as_str().xpect_eq("title");
		fields[1].1.xpect_eq(DataLiteral::Scalar(Value::Uint(2)));
	}

	/// A `[Section]` header declares its own component alongside the default.
	#[beet_core::test]
	fn declares_sectioned_components() {
		let fm = Frontmatter::parse(
			"title = \"Hello\"\n[Extra]\nnote = \"hi\"",
			FrontmatterKind::Toml,
		)
		.unwrap();
		fm.declarations("PageMeta")
			.0
			.iter()
			.map(|named| named.name.to_string())
			.collect::<Vec<_>>()
			.xpect_eq(vec!["PageMeta".to_string(), "Extra".to_string()]);
		// only the default component's keys reach the reflect surface
		fm.get_str("title").unwrap().xpect_eq("Hello");
		fm.get_str("note").is_none().xpect_true();
	}

	// -- extraction from raw source --

	#[beet_core::test]
	fn extract_toml_fence() {
		let fm = Frontmatter::extract(
			"+++\ntitle = \"Hello\"\norder = 2\n+++\n\n# Body",
		)
		.unwrap()
		.unwrap();
		fm.kind.xpect_eq(FrontmatterKind::Toml);
		fm.get_str("title").unwrap().xpect_eq("Hello");
		fm.get_uint("order").unwrap().xpect_eq(2);
	}

	#[beet_core::test]
	fn extract_yaml_fence() {
		let fm = Frontmatter::extract("---\ntitle: Hello\n---\n# Body")
			.unwrap()
			.unwrap();
		fm.kind.xpect_eq(FrontmatterKind::Yaml);
		fm.get_str("title").unwrap().xpect_eq("Hello");
	}

	#[beet_core::test]
	fn extract_none() {
		// no fence at all
		Frontmatter::extract("# Body")
			.unwrap()
			.is_none()
			.xpect_true();
		// a thematic break is not a fence
		Frontmatter::extract("--- not a fence")
			.unwrap()
			.is_none()
			.xpect_true();
		// an unclosed fence is not frontmatter
		Frontmatter::extract("---\ntitle: Hello")
			.unwrap()
			.is_none()
			.xpect_true();
	}

	// -- Frontmatter component --

	#[beet_core::test]
	fn frontmatter_yaml() {
		let fm = Frontmatter::parse(
			"title: Hello\nauthor: World",
			FrontmatterKind::Yaml,
		)
		.unwrap();
		fm.kind.xpect_eq(FrontmatterKind::Yaml);
		fm.value.field_len().xpect_eq(2);
	}

	#[beet_core::test]
	fn frontmatter_toml() {
		let fm = Frontmatter::parse(
			"title = \"Hello\"\ncount = 42",
			FrontmatterKind::Toml,
		)
		.unwrap();
		fm.kind.xpect_eq(FrontmatterKind::Toml);
		fm.value.field_len().xpect_eq(2);
	}

	#[beet_core::test]
	fn frontmatter_empty() {
		let fm = Frontmatter::parse("", FrontmatterKind::Yaml).unwrap();
		fm.value.field_len().xpect_eq(0);
	}

	#[beet_core::test]
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

	#[beet_core::test]
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
