//! Where a document's entities were written: [`SourceMap`], the
//! [`ParsedDocument`] it sits beside, and the [`SourcePosition`] an error or a
//! diagnostic names.
//!
//! A position is never component data, so whitespace changes no body and no
//! content id. A parser counts byte offsets, and a line and column are
//! computed from the source when one is shown.
use crate::prelude::*;
use core::ops::Range;
use thiserror::Error;

/// Where each entity of a document was written, the byte range it was authored
/// at. Empty where its format has no positions.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SourceMap {
	/// The range of each entity, by its key in the body.
	pub spans: HashMap<EntityKey, Range<u32>>,
}

impl SourceMap {
	/// Record that the entity keyed `key` was written at `span`.
	pub fn insert(&mut self, key: EntityKey, span: Range<u32>) {
		self.spans.insert(key, span);
	}

	/// Count every span from `offset` further back, for a document that is
	/// part of another.
	pub fn shift(&mut self, offset: u32) {
		for span in self.spans.values_mut() {
			*span = span.start + offset..span.end + offset;
		}
	}

	/// Where the entity keyed `key` was written.
	pub fn get(&self, key: &EntityKey) -> Option<&Range<u32>> {
		self.spans.get(key)
	}

	/// The spans the reader of a structured format gives of the scene body
	/// `bytes` are: TOML's, each entity its own table's, json's and postcard's
	/// giving none.
	pub fn of_structured(bytes: &MediaBytes) -> Result<Self> {
		match bytes.media_type() {
			#[cfg(feature = "toml")]
			MediaType::Toml => Self::of_toml(bytes.as_utf8()?).xok(),
			_ => Self::default().xok(),
		}
	}

	/// The spans of the entities a TOML scene body holds, none for any other
	/// document.
	#[cfg(feature = "toml")]
	fn of_toml(text: &str) -> Self {
		let Ok(table) = toml::de::DeTable::parse(text) else {
			return default();
		};
		let Some(entities) = table
			.get_ref()
			.get("entities")
			.and_then(|entities| entities.get_ref().as_table())
		else {
			return default();
		};
		Self {
			spans: entities
				.iter()
				.filter_map(|(key, value)| {
					let span = value.span();
					EntityKey::parse(key.get_ref())
						.ok()
						.map(|key| (key, span.start as u32..span.end as u32))
				})
				.collect(),
		}
	}
}

/// A document as its parser read it: its scene body and, beside it, where each
/// entity of it was written.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ParsedDocument {
	/// The scene the document is.
	pub body: SceneBody,
	/// Where its entities were written.
	pub spans: SourceMap,
}

/// A body whose format has no positions.
impl From<SceneBody> for ParsedDocument {
	fn from(body: SceneBody) -> Self {
		Self {
			body,
			spans: default(),
		}
	}
}

/// A document as it was read, and what it was read from, so whatever it
/// becomes answers where it was written.
#[derive(Debug, Clone, Get)]
pub struct DocumentSource {
	/// The document's path in its package, if it has one.
	#[get(skip)]
	path: Option<RelPath>,
	/// What it was read from.
	bytes: MediaBytes,
	#[get(skip)]
	document: ParsedDocument,
}

impl DocumentSource {
	/// `document`, read from `bytes`, the file at `path` of its package.
	pub fn new(
		path: Option<RelPath>,
		bytes: MediaBytes,
		document: ParsedDocument,
	) -> Self {
		Self {
			path,
			bytes,
			document,
		}
	}

	/// The document's path in its package, if it has one.
	pub fn path(&self) -> Option<&RelPath> { self.path.as_ref() }

	/// The scene the document is.
	pub fn body(&self) -> &SceneBody { &self.document.body }

	/// Where its entities were written.
	pub fn spans(&self) -> &SourceMap { &self.document.spans }

	/// Where the entity keyed `key` was written, `None` when the format says
	/// nothing of it.
	pub fn position(&self, key: &EntityKey) -> Option<SourcePosition> {
		self.spans().get(key).map(|span| {
			SourcePosition::new(self.path(), &self.bytes, span.start)
		})
	}
}

/// A place in a document as a person reads one, `routes/index.md:3:5`: its
/// path where it has one, and its line and column where they are known.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SourcePosition {
	/// The document's path in its package.
	pub path: Option<RelPath>,
	/// The line and column, [`LineCol`]'s column counting from zero.
	pub line_col: Option<LineCol>,
}

impl SourcePosition {
	/// The place `offset` bytes into `source`, the document at `path`.
	pub fn new(path: Option<&RelPath>, source: &[u8], offset: u32) -> Self {
		let before = &source[..(offset as usize).min(source.len())];
		let line = before.iter().filter(|byte| **byte == b'\n').count() + 1;
		let start = before
			.iter()
			.rposition(|byte| *byte == b'\n')
			.map_or(0, |newline| newline + 1);
		// a column counts characters, each one leading byte
		let col = before[start..]
			.iter()
			.filter(|byte| !matches!(byte, 0x80..0xC0))
			.count();
		Self {
			path: path.cloned(),
			line_col: Some(LineCol::new(line as u32, col as u32)),
		}
	}

	/// The document at `path`, nowhere in particular.
	pub fn document(path: RelPath) -> Self {
		Self {
			path: Some(path),
			line_col: None,
		}
	}
}

/// `path:line:col`, the column counted from one, less whatever is unknown.
impl core::fmt::Display for SourcePosition {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match (&self.path, &self.line_col) {
			(Some(path), Some(at)) => {
				write!(f, "{path}:{}:{}", at.line, at.col + 1)
			}
			(Some(path), None) => write!(f, "{path}"),
			(None, Some(at)) => write!(f, "{}:{}", at.line, at.col + 1),
			(None, None) => Ok(()),
		}
	}
}

/// An error at a byte offset of the document being read, which the reader
/// holding its source shows as a [`SourcePosition`].
#[derive(Clone, Error)]
#[error("{error}")]
pub struct SourceError {
	offset: u32,
	error: CloneError,
}

/// As its inner error reads, the offset meaning nothing without its source.
impl core::fmt::Debug for SourceError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		core::fmt::Debug::fmt(&self.error, f)
	}
}

impl SourceError {
	/// `error` at `offset`, unless it already knows where it is: the innermost
	/// place is the nearest.
	pub fn at(error: BevyError, offset: usize) -> BevyError {
		match error.downcast_ref::<Self>() {
			Some(_) => error,
			None => Self {
				offset: offset as u32,
				error: CloneError::new(error),
			}
			.into(),
		}
	}

	/// `error` with its place counted from `offset` further back, for one read
	/// from a slice of its document, else at `offset` itself.
	pub fn within(error: BevyError, offset: usize) -> BevyError {
		match error.downcast_ref::<Self>().cloned() {
			Some(inner) => Self {
				offset: inner.offset + offset as u32,
				error: inner.error,
			}
			.into(),
			None => Self::at(error, offset),
		}
	}

	/// The place `error` knows it is at, if it does, and the error itself.
	pub fn split(error: BevyError) -> (Option<u32>, BevyError) {
		match error.downcast_ref::<Self>().cloned() {
			Some(inner) => (Some(inner.offset), inner.error.into()),
			None => (None, error),
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn a_position_counts_from_one() {
		let show = |offset: u32| {
			SourcePosition::new(
				Some(&RelPath::new("a.md")),
				"één\nb\u{1F331}c".as_bytes(),
				offset,
			)
			.to_string()
		};
		show(0).xpect_eq("a.md:1:1");
		// a column is a character, however many bytes it takes
		show(2).xpect_eq("a.md:1:2");
		show(6).xpect_eq("a.md:2:1");
		show(11).xpect_eq("a.md:2:3");
		// past the end is the end
		show(99).xpect_eq("a.md:2:4");
		SourcePosition::document(RelPath::new("a.md"))
			.to_string()
			.xpect_eq("a.md");
	}

	/// The innermost place wins, and a slice's counts from its own start.
	#[crate::test]
	fn the_innermost_place_wins() {
		let error = SourceError::at(bevyhow!("nope"), 4);
		let error = SourceError::at(error, 9);
		let (offset, error) =
			SourceError::split(SourceError::within(error, 10));
		offset.xpect_eq(Some(14));
		error.to_string().trim().xpect_contains("nope");
		SourceError::split(bevyhow!("nope")).0.xpect_none();
	}
}
