//! [`MediaType::Yaml`]: the YAML a frontmatter block is written in, read and
//! written by beet itself through [`Value`], since no serde YAML crate is
//! reliably maintained.
//!
//! The subset is what frontmatter uses, and anything else is refused with an
//! error naming it rather than read some other way:
//!
//! - a document is a map of `key: value` lines, a block list of `- item`
//!   lines, or one scalar;
//! - a value is a scalar, a flow list `[a, "b, c", 3]`, a block list of `-`
//!   lines beneath its key, or one level of nested map, indented beneath its
//!   key;
//! - a scalar is plain, typed as [`Value::parse_string`] types it (`42`,
//!   `true`, `3.5`, `~` or `null`, else text), or quoted, `"..."` with
//!   backslash escapes or `'...'` with `''` for a quote, always text;
//! - a `#` comment runs to the line's end, outside quotes and after
//!   whitespace.
//!
//! Refused: anchors and aliases, tags, block scalars (`|`, `>`), flow maps,
//! nested lists, lists of maps, maps nested more than one level, a second
//! document and tabs in indentation.
//!
//! ```
//! # use beet_core::prelude::*;
//! let value = MediaType::Yaml
//! 	.deserialize::<Value>(b"title: Plan\ntags: [ecs, web]\n")
//! 	.unwrap();
//! value.get("title").cloned().unwrap().xpect_eq(Value::str("Plan"));
//! MediaType::Yaml
//! 	.deserialize::<Value>(b"logo: &shared x\n")
//! 	.unwrap_err()
//! 	.to_string()
//! 	.xpect_contains("an anchor or alias");
//! ```
use crate::prelude::*;

/// The reader and writer behind [`MediaType::Yaml`], see the [module](self)
/// docs.
pub(crate) struct Yaml;

/// One line of a document holding content: its indent, its text with any
/// comment and trailing whitespace removed, and its number from 1.
struct Line<'a> {
	indent: usize,
	text: &'a str,
	number: usize,
}

impl Yaml {
	/// Reads `text` into a [`Value`]: a map, a list or a scalar, an empty
	/// document being an empty map.
	pub fn read(text: &str) -> Result<Value> {
		let lines = Self::lines(text)?;
		let Some(first) = lines.first() else {
			return Value::Map(default()).xok();
		};
		let mut position = 0;
		let value = match () {
			_ if Self::is_item(first.text) => Value::List(Self::read_list(
				&lines,
				&mut position,
				first.indent,
			)?),
			_ if Self::split_key(first.text).is_some() => Value::Map(
				Self::read_map(&lines, &mut position, first.indent, 0)?,
			),
			_ => {
				position = 1;
				Self::read_value(first)?
			}
		};
		match lines.get(position) {
			Some(line) => bevybail!(
				"YAML line {}: `{}` follows a complete document",
				line.number,
				line.text
			),
			None => value.xok(),
		}
	}

	/// Writes `value` as the subset reads it back: a map's entries in their
	/// order, a list of scalars inline, a nested map indented beneath its key.
	pub fn write(value: &Value) -> Result<String> {
		let mut out = String::new();
		match value {
			Value::Map(map) => {
				for (key, value) in map.0.iter() {
					Self::write_entry(&mut out, "", key, value, 0)?;
				}
			}
			Value::List(items) => {
				for item in items {
					out.push_str(&format!("- {}\n", Self::scalar(item)?));
				}
			}
			value => {
				out.push_str(&Self::scalar(value)?);
				out.push('\n');
			}
		}
		out.xok()
	}

	/// One `key: value` entry at `indent`, a nested map at `depth` 0 written
	/// beneath it.
	fn write_entry(
		out: &mut String,
		indent: &str,
		key: &str,
		value: &Value,
		depth: usize,
	) -> Result {
		let key = Self::string(key, false);
		match value {
			Value::Map(map) if depth == 0 => {
				out.push_str(&format!("{indent}{key}:\n"));
				for (sub, value) in map.0.iter() {
					Self::write_entry(out, "  ", sub, value, depth + 1)?;
				}
			}
			Value::Map(_) => {
				bevybail!("YAML `{key}`: a map nested more than one level deep")
			}
			value => {
				out.push_str(&format!(
					"{indent}{key}: {}\n",
					Self::flow(value)?
				));
			}
		}
		Ok(())
	}

	/// A value on its key's line: a scalar, or a list of scalars inline.
	fn flow(value: &Value) -> Result<String> {
		match value {
			Value::List(items) => items
				.iter()
				.map(|item| match item {
					Value::Str(text) => Self::string(text, true).xok(),
					Value::List(_) | Value::Map(_) => {
						bevybail!("YAML: a list holding a {}", item.kind())
					}
					item => Self::scalar(item),
				})
				.collect::<Result<Vec<_>>>()?
				.join(", ")
				.xmap(|items| format!("[{items}]"))
				.xok(),
			value => Self::scalar(value),
		}
	}

	/// A scalar as it reads back.
	fn scalar(value: &Value) -> Result<String> {
		match value {
			Value::Null => "null".to_string().xok(),
			Value::Str(text) => Self::string(text, false).xok(),
			Value::Bool(_)
			| Value::Int(_)
			| Value::Uint(_)
			| Value::Float(_) => value.to_string().xok(),
			other => bevybail!("YAML: a {} is no scalar", other.kind()),
		}
	}

	/// A string on one line, quoted where plain it would read as another type,
	/// lose its edges or comment, or split or close a list it is an item of.
	fn string(text: &str, in_list: bool) -> String {
		let text = text.replace(['\r', '\n'], " ");
		let is_plain = !text.is_empty()
			&& Self::plain(&text) == Value::str(text.as_str())
			&& text.trim() == text
			&& !text.contains(" #")
			&& !text.starts_with([
				'&', '*', '!', '|', '>', '[', '{', '"', '\'', '#', '-',
			]) && !text.ends_with(':')
			&& !(in_list && text.contains([',', '[', ']']));
		match is_plain {
			true => text,
			false => format!(
				"\"{}\"",
				text.replace('\\', "\\\\").replace('"', "\\\"")
			),
		}
	}

	/// The lines holding content, each with its indent and its comment
	/// removed, refusing a tab in indentation and a second document.
	fn lines(text: &str) -> Result<Vec<Line<'_>>> {
		let mut lines = Vec::new();
		for (index, raw) in text.lines().enumerate() {
			let number = index + 1;
			let content = raw.trim_start_matches(' ');
			if content.starts_with('\t') {
				bevybail!("YAML line {number}: a tab in indentation");
			}
			let text = Self::strip_comment(content).trim_end();
			match text {
				"" => {}
				"---" | "..." if index > 0 || text == "..." => {
					bevybail!("YAML line {number}: a second document")
				}
				"---" => {}
				text => lines.push(Line {
					indent: raw.len() - content.len(),
					text,
					number,
				}),
			}
		}
		lines.xok()
	}

	/// `text` up to a `#` that opens a comment: at its start, or after
	/// whitespace, outside quotes.
	fn strip_comment(text: &str) -> &str {
		let mut quote = None;
		let mut previous = ' ';
		for (index, char) in text.char_indices() {
			match (quote, char) {
				(None, '"' | '\'')
					if previous == ' '
						|| previous == '[' || previous == ',' =>
				{
					quote = Some(char)
				}
				(Some('"'), '\\') => {}
				(Some(open), char) if char == open && previous != '\\' => {
					quote = None
				}
				(None, '#') if previous.is_whitespace() => {
					return &text[..index];
				}
				_ => {}
			}
			previous = char;
		}
		text
	}

	/// Whether a line is a block list's item.
	fn is_item(text: &str) -> bool { text == "-" || text.starts_with("- ") }

	/// A map line's key and the rest after its `:`, which is followed by a
	/// space or ends the line; `None` for a line that is no `key:` line.
	fn split_key(text: &str) -> Option<(&str, &str)> {
		let mut quote = None;
		for (index, char) in text.char_indices() {
			match (quote, char) {
				(None, '"' | '\'') if index == 0 => quote = Some(char),
				(Some(open), char) if char == open => quote = None,
				(None, ':') => {
					let rest = &text[index + 1..];
					if rest.is_empty() || rest.starts_with(' ') {
						return Some((text[..index].trim(), rest.trim()));
					}
				}
				(None, '[' | '{') if index == 0 => return None,
				_ => {}
			}
		}
		None
	}

	/// The map whose keys sit at `indent`, starting at `position`, at `depth`
	/// nested maps from the document's.
	fn read_map(
		lines: &[Line],
		position: &mut usize,
		indent: usize,
		depth: usize,
	) -> Result<Map> {
		let mut map = Map::default();
		while let Some(line) = lines.get(*position) {
			if line.indent < indent {
				break;
			}
			if line.indent > indent {
				bevybail!("YAML line {}: unexpected indentation", line.number);
			}
			if Self::is_item(line.text) {
				bevybail!(
					"YAML line {}: a list item where a key was expected",
					line.number
				);
			}
			let (key, rest) = Self::split_key(line.text).ok_or_else(|| {
				bevyhow!(
					"YAML line {}: `{}` has no key",
					line.number,
					line.text
				)
			})?;
			let key = SmolStr::new(Self::unquote_key(key, line.number)?);
			*position += 1;
			let value = match (rest.is_empty(), lines.get(*position)) {
				(false, _) => Self::read_value(&Line {
					indent: line.indent,
					text: rest,
					number: line.number,
				})?,
				(true, Some(next))
					if Self::is_item(next.text) && next.indent >= indent =>
				{
					Value::List(Self::read_list(lines, position, next.indent)?)
				}
				(true, Some(next)) if next.indent > indent => {
					if depth >= 1 {
						bevybail!(
							"YAML line {}: a map nested more than one level deep",
							next.number
						);
					}
					Value::Map(Self::read_map(
						lines,
						position,
						next.indent,
						depth + 1,
					)?)
				}
				(true, _) => Value::Null,
			};
			if map.0.insert(key.clone(), value).is_some() {
				bevybail!("YAML line {}: `{key}` appears twice", line.number);
			}
		}
		map.xok()
	}

	/// The block list whose items sit at `indent`, starting at `position`.
	fn read_list(
		lines: &[Line],
		position: &mut usize,
		indent: usize,
	) -> Result<Vec<Value>> {
		let mut items = Vec::new();
		while let Some(line) = lines.get(*position)
			&& line.indent == indent
			&& Self::is_item(line.text)
		{
			let item = line.text[1..].trim();
			if item.is_empty() || Self::is_item(item) {
				bevybail!("YAML line {}: a nested list", line.number);
			}
			if Self::split_key(item).is_some() && !item.starts_with(['"', '\''])
			{
				bevybail!("YAML line {}: a list of maps", line.number);
			}
			items.push(Self::read_value(&Line {
				indent: line.indent,
				text: item,
				number: line.number,
			})?);
			*position += 1;
		}
		if let Some(line) = lines.get(*position)
			&& line.indent > indent
		{
			bevybail!("YAML line {}: unexpected indentation", line.number);
		}
		items.xok()
	}

	/// A value written on one line: a flow list or a scalar.
	fn read_value(line: &Line) -> Result<Value> {
		let text = line.text;
		let refuse = |what: &str| -> Result<Value> {
			bevybail!("YAML line {}: {what} is not supported", line.number)
		};
		match text.chars().next() {
			Some('[') => Self::read_flow(line),
			Some('{') => refuse("a flow map"),
			Some('&' | '*') => refuse("an anchor or alias"),
			Some('!') => refuse("a tag"),
			Some('|' | '>') => refuse("a block scalar"),
			_ => Self::read_scalar(text, line.number),
		}
	}

	/// A `[a, b]` list of scalars.
	fn read_flow(line: &Line) -> Result<Value> {
		let inner = line
			.text
			.strip_prefix('[')
			.and_then(|inner| inner.strip_suffix(']'))
			.ok_or_else(|| {
				bevyhow!("YAML line {}: an unclosed flow list", line.number)
			})?;
		let mut items = Vec::new();
		let mut quote = None;
		let mut start = 0;
		for (index, char) in inner.char_indices() {
			match (quote, char) {
				(None, '"' | '\'') => quote = Some(char),
				(Some(open), char) if char == open => quote = None,
				(None, '[' | '{') => bevybail!(
					"YAML line {}: a nested flow collection is not supported",
					line.number
				),
				(None, ',') => {
					items.push(&inner[start..index]);
					start = index + 1;
				}
				_ => {}
			}
		}
		items.push(&inner[start..]);
		items
			.into_iter()
			.map(str::trim)
			.filter(|item| !item.is_empty())
			.map(|item| Self::read_scalar(item, line.number))
			.collect::<Result<Vec<_>>>()?
			.xmap(Value::List)
			.xok()
	}

	/// A scalar: quoted text, or plain and typed.
	fn read_scalar(text: &str, number: usize) -> Result<Value> {
		let quoted = |close: char| -> Result<&str> {
			text[1..].strip_suffix(close).ok_or_else(|| {
				bevyhow!("YAML line {number}: an unclosed quote in `{text}`")
			})
		};
		match text.chars().next() {
			Some('"') => {
				Self::unescape(quoted('"')?, number)?.xmap(Value::str).xok()
			}
			Some('\'') => Value::str(quoted('\'')?.replace("''", "'")).xok(),
			_ => Self::plain(text).xok(),
		}
	}

	/// A plain scalar's value, typed.
	fn plain(text: &str) -> Value {
		match text {
			"~" | "null" | "Null" | "NULL" => Value::Null,
			text => Value::parse_string(text),
		}
	}

	/// A key, its quotes removed.
	fn unquote_key(key: &str, number: usize) -> Result<String> {
		match key.chars().next() {
			Some('"' | '\'') => match Self::read_scalar(key, number)? {
				Value::Str(text) => text.to_string().xok(),
				_ => key.to_string().xok(),
			},
			_ => key.to_string().xok(),
		}
	}

	/// A double-quoted scalar's escapes resolved.
	fn unescape(text: &str, number: usize) -> Result<String> {
		let mut out = String::new();
		let mut chars = text.chars();
		while let Some(char) = chars.next() {
			if char != '\\' {
				out.push(char);
				continue;
			}
			match chars.next() {
				Some('n') => out.push('\n'),
				Some('t') => out.push('\t'),
				Some(char @ ('\\' | '"' | '/')) => out.push(char),
				other => bevybail!(
					"YAML line {number}: the escape `\\{}` is not supported",
					other.map(String::from).unwrap_or_default()
				),
			}
		}
		out.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	fn read(text: &str) -> Value {
		MediaType::Yaml
			.deserialize::<Value>(text.as_bytes())
			.unwrap()
	}

	fn refusal(text: &str) -> String {
		MediaType::Yaml
			.deserialize::<Value>(text.as_bytes())
			.unwrap_err()
			.to_string()
	}

	/// Every form of the subset reads, its scalars typed as frontmatter types
	/// them and its comments dropped.
	#[crate::test]
	fn reads_the_subset() {
		let value = read(
			"# a comment\ntitle: Plan: one, two\ncount: 42 # inline\n\
			 offset: -7\nweight: 3.5\ndraft: true\nnone: ~\nempty:\n\
			 quoted: \"a \\\"b\\\" # c\"\nsingle: 'it''s'\n\
			 tags: [ecs, \"web, http\", 3]\nblock:\n  - one\n  - 'two'\n\
			 flush:\n- a\nExtra:\n  note: hi\n  sizes: [16, 32]\n",
		);
		let get = |key: &str| value.get(key).cloned().unwrap();
		get("title").xpect_eq(Value::str("Plan: one, two"));
		get("count").xpect_eq(Value::Uint(42));
		get("offset").xpect_eq(Value::Int(-7));
		get("weight").xpect_eq(Value::Float(3.5));
		get("draft").xpect_eq(Value::Bool(true));
		get("none").xpect_eq(Value::Null);
		get("empty").xpect_eq(Value::Null);
		get("quoted").xpect_eq(Value::str("a \"b\" # c"));
		get("single").xpect_eq(Value::str("it's"));
		get("tags").xpect_eq(Value::List(vec![
			Value::str("ecs"),
			Value::str("web, http"),
			Value::Uint(3),
		]));
		get("block")
			.xpect_eq(Value::List(vec![Value::str("one"), Value::str("two")]));
		get("flush").xpect_eq(Value::List(vec![Value::str("a")]));
		get("Extra")
			.get("note")
			.cloned()
			.unwrap()
			.xpect_eq(Value::str("hi"));
		read("").xpect_eq(Value::Map(default()));
		read("- a\n- 2\n")
			.xpect_eq(Value::List(vec![Value::str("a"), Value::Uint(2)]));
	}

	/// Anything beyond the subset is refused, named.
	#[crate::test]
	fn refuses_beyond_the_subset() {
		for (text, named) in [
			("a: &x 1\n", "an anchor or alias"),
			("a: *x\n", "an anchor or alias"),
			("a: !tag 1\n", "a tag"),
			("a: |\n  text\n", "a block scalar"),
			("a: {b: 1}\n", "a flow map"),
			("a: [[1]]\n", "a nested flow collection"),
			("a:\n  - - 1\n", "a nested list"),
			("a:\n  - b: 1\n", "a list of maps"),
			("a:\n  b:\n    c: 1\n", "nested more than one level"),
			("a: 1\n---\nb: 2\n", "a second document"),
			("a:\n\tb: 1\n", "a tab in indentation"),
			("a: 1\na: 2\n", "appears twice"),
			("a: \"open\n", "an unclosed quote"),
		] {
			refusal(text).xpect_contains(named);
		}
	}

	/// What the writer writes reads back as the value it wrote.
	#[crate::test]
	fn writes_what_it_reads() {
		#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
		struct Meta {
			title: String,
			authors: Vec<String>,
			order: u32,
			note: Option<String>,
			flag: String,
		}
		let meta = Meta {
			title: "Plan: one, two".into(),
			authors: vec!["Mel".into(), "Packwood, Geoff".into()],
			order: 2,
			note: None,
			flag: "true".into(),
		};
		let bytes = MediaType::Yaml.serialize(&meta).unwrap();
		String::from_utf8(bytes.clone()).unwrap().xpect_eq(
			"title: Plan: one, two\nauthors: [Mel, \"Packwood, Geoff\"]\norder: 2\nnote: null\nflag: \"true\"\n",
		);
		MediaType::Yaml
			.deserialize::<Meta>(&bytes)
			.unwrap()
			.xpect_eq(meta);
	}
}
