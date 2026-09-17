//! The `.env` grammar: flat `KEY=value` lines, the one format that becomes a
//! process environment.

use crate::prelude::*;
use core::fmt;

/// A `.env` file as an ordered list of lines: blank, `#` comment, or a
/// `KEY=value` pair, every line kept as written so an edit changes one line
/// and nothing else. The one dotenv grammar in beet: `env_ext::parse_dotenv`
/// and the `.env.age` vault format both read through it, so the two cannot
/// drift.
///
/// The grammar: a leading `export ` is dropped, the key and value are
/// trimmed, a value wrapped in matching single or double quotes is unwrapped
/// (`\"` and `\\` unescape inside double quotes, a single-quoted value is
/// literal), and a line without a `=` is kept verbatim but yields no pair.
///
/// ## Example
///
/// ```
/// # use beet_core::prelude::*;
/// let mut doc = EnvDocument::parse("# api\nKEY=old\n");
/// doc.set("KEY", "new");
/// doc.set("OTHER", "a b");
/// doc.to_string().xpect_eq("# api\nKEY=new\nOTHER=\"a b\"\n");
/// doc.get("OTHER").xpect_eq(Some("a b"));
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct EnvDocument {
	lines: Vec<EnvLine>,
}

/// One line of a `.env` file.
#[derive(Debug, Clone, PartialEq, Eq)]
enum EnvLine {
	/// A blank, a comment, or a line that is not a pair, kept as written.
	Text(String),
	/// A `KEY=value` pair, with the line it was read from (or rendered as)
	/// so an untouched pair round-trips byte for byte.
	Pair {
		key: SmolStr,
		value: SmolStr,
		text: String,
	},
}

impl EnvDocument {
	/// Parse `.env` contents. Never fails: a line the grammar does not
	/// recognize is kept as text.
	pub fn parse(contents: &str) -> Self {
		Self {
			lines: contents.lines().map(EnvLine::parse).collect(),
		}
	}

	/// The value of `key`, the last one when it repeats (as a shell would).
	pub fn get(&self, key: &str) -> Option<&str> {
		self.lines
			.iter()
			.rev()
			.filter_map(EnvLine::pair)
			.find(|(name, _)| *name == key)
			.map(|(_, value)| value)
	}

	/// Set `key` to `value`, replacing the pair in place when it exists (a
	/// repeat dropped, so one key means one value), else appending one.
	pub fn set(&mut self, key: impl Into<SmolStr>, value: impl Into<SmolStr>) {
		let key = key.into();
		let value = value.into();
		let is_key = |line: &EnvLine| {
			line.pair().is_some_and(|(name, _)| name == key.as_str())
		};
		match self.lines.iter().position(is_key) {
			Some(index) => {
				// the first occurrence takes the value, any repeat goes
				let mut seen = 0;
				self.lines.retain(|line| {
					seen += is_key(line) as usize;
					seen <= 1 || !is_key(line)
				});
				self.lines[index] = EnvLine::render(key, value);
			}
			None => self.lines.push(EnvLine::render(key, value)),
		}
	}

	/// Remove every pair named `key`, `true` when one existed.
	pub fn remove(&mut self, key: &str) -> bool {
		let before = self.lines.len();
		self.lines
			.retain(|line| !line.pair().is_some_and(|(name, _)| name == key));
		self.lines.len() != before
	}

	/// Every key, in order, deduplicated.
	pub fn keys(&self) -> Vec<SmolStr> {
		let mut keys = Vec::new();
		for (key, _) in self.pairs() {
			if !keys.contains(&key) {
				keys.push(key);
			}
		}
		keys
	}

	/// Every pair in file order, a repeated key repeated.
	pub fn pairs(&self) -> Vec<(SmolStr, SmolStr)> {
		self.lines
			.iter()
			.filter_map(EnvLine::pair)
			.map(|(key, value)| (SmolStr::new(key), SmolStr::new(value)))
			.collect()
	}

	/// Whether the document holds no pair.
	pub fn is_empty(&self) -> bool { self.pairs().is_empty() }

	/// Merge every pair of `other` in, an existing key kept unless
	/// `replace`. Returns the keys written.
	pub fn merge(&mut self, other: &Self, replace: bool) -> Vec<SmolStr> {
		let mut written = Vec::new();
		for (key, value) in other.pairs() {
			if replace || self.get(&key).is_none() {
				self.set(key.clone(), value);
				written.push(key);
			}
		}
		written
	}
}

impl EnvLine {
	/// Parse one line: a pair when it has a `=` (after an optional
	/// `export `), else text.
	fn parse(line: &str) -> Self {
		let trimmed = line.trim();
		if trimmed.is_empty() || trimmed.starts_with('#') {
			return Self::Text(line.to_string());
		}
		let Some((key, value)) = trimmed
			.strip_prefix("export ")
			.unwrap_or(trimmed)
			.split_once('=')
		else {
			return Self::Text(line.to_string());
		};
		Self::Pair {
			key: SmolStr::new(key.trim()),
			value: SmolStr::new(Self::unquote(value.trim())),
			text: line.to_string(),
		}
	}

	/// A value with its matching quotes removed: a double-quoted value
	/// unescapes `\"` and `\\`, a single-quoted one is literal, an unquoted
	/// one is as written.
	fn unquote(value: &str) -> String {
		match value.as_bytes() {
			[b'"', .., b'"'] => value[1..value.len() - 1]
				.replace("\\\"", "\"")
				.replace("\\\\", "\\"),
			[b'\'', .., b'\''] => value[1..value.len() - 1].to_string(),
			_ => value.to_string(),
		}
	}

	/// A pair rendered as `KEY=value`, double-quoted (with `"` and `\`
	/// escaped) when the value would not survive the grammar bare.
	fn render(key: SmolStr, value: SmolStr) -> Self {
		let needs_quotes = value.is_empty()
			|| value.chars().any(|ch| {
				ch.is_whitespace() || matches!(ch, '#' | '"' | '\'' | '\\')
			});
		let text = match needs_quotes {
			true => format!(
				"{key}=\"{}\"",
				value.replace('\\', "\\\\").replace('"', "\\\"")
			),
			false => format!("{key}={value}"),
		};
		Self::Pair { key, value, text }
	}

	/// The pair this line holds, if any.
	fn pair(&self) -> Option<(&str, &str)> {
		match self {
			Self::Pair { key, value, .. } => Some((key, value)),
			Self::Text(_) => None,
		}
	}
}

/// The file's text, one line each with a trailing newline.
impl fmt::Display for EnvDocument {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		for line in &self.lines {
			match line {
				EnvLine::Text(text) => writeln!(f, "{text}")?,
				EnvLine::Pair { text, .. } => writeln!(f, "{text}")?,
			}
		}
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	const SOURCE: &str = "# a comment\n\nFOO=bar\nexport BAZZ='boo'\nBOOM=\"a b\"\nURL=http://x?a=b\nnot a pair\nESC=\"say \\\"hi\\\"\"\n";

	// comments, blanks, `export`, quoting and an `=` inside a value
	#[crate::test]
	fn parses_the_grammar() {
		let doc = EnvDocument::parse(SOURCE);
		doc.pairs().xpect_eq(vec![
			(SmolStr::new("FOO"), SmolStr::new("bar")),
			(SmolStr::new("BAZZ"), SmolStr::new("boo")),
			(SmolStr::new("BOOM"), SmolStr::new("a b")),
			(SmolStr::new("URL"), SmolStr::new("http://x?a=b")),
			(SmolStr::new("ESC"), SmolStr::new("say \"hi\"")),
		]);
		doc.get("BOOM").xpect_eq(Some("a b"));
		doc.get("missing").xpect_none();
	}

	// an untouched document renders byte for byte
	#[crate::test]
	fn roundtrips_verbatim() {
		EnvDocument::parse(SOURCE).to_string().xpect_eq(SOURCE);
	}

	#[crate::test]
	fn set_replaces_in_place_else_appends() {
		let mut doc = EnvDocument::parse("A=1\n# note\nB=2\n");
		doc.set("B", "two words");
		doc.set("C", "");
		doc.set("D", "it's");
		doc.to_string()
			.xpect_eq("A=1\n# note\nB=\"two words\"\nC=\"\"\nD=\"it's\"\n");
		// the rendered form parses back to the value
		EnvDocument::parse(&doc.to_string())
			.get("D")
			.xpect_eq(Some("it's"));
	}

	#[crate::test]
	fn remove_and_keys() {
		let mut doc = EnvDocument::parse("A=1\nB=2\nA=3\n");
		doc.keys()
			.xpect_eq(vec![SmolStr::new("A"), SmolStr::new("B")]);
		// the last occurrence wins, as a shell would
		doc.get("A").xpect_eq(Some("3"));
		doc.set("A", "4");
		doc.to_string().xpect_eq("A=4\nB=2\n");
		doc.remove("A").xpect_true();
		doc.remove("A").xpect_false();
		doc.to_string().xpect_eq("B=2\n");
	}

	#[crate::test]
	fn merge_keeps_existing_unless_replacing() {
		let mut doc = EnvDocument::parse("A=1\n");
		let other = EnvDocument::parse("A=9\nB=2\n");
		doc.merge(&other, false).xpect_eq(vec![SmolStr::new("B")]);
		doc.get("A").xpect_eq(Some("1"));
		doc.merge(&other, true).len().xpect_eq(2);
		doc.get("A").xpect_eq(Some("9"));
	}
}
