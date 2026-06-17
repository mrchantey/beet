//! A minimal character cursor over a `&str`, the primitive the hand-written
//! recursive-descent BSX parser is built on.
//!
//! It tracks a byte offset into the source and exposes the small peek/bump/eat
//! vocabulary the markup, value, and spread parsers share. Spans are byte
//! offsets so [`FileSpan`](beet_core::prelude::FileSpan) tracking can resolve
//! them through a [`SpanLookup`](crate::prelude::SpanLookup) later.

/// A forward-only character cursor over the BSX source.
pub struct Cursor<'a> {
	/// The full source text.
	source: &'a str,
	/// The current byte offset into [`source`](Self::source).
	offset: usize,
}

impl<'a> Cursor<'a> {
	/// Create a cursor at the start of `source`.
	pub fn new(source: &'a str) -> Self { Self { source, offset: 0 } }

	/// The unconsumed remainder of the source.
	pub fn rest(&self) -> &'a str { &self.source[self.offset..] }

	/// The current byte offset into the source.
	pub fn offset(&self) -> usize { self.offset }

	/// The [`LineCol`] of a byte `offset` into the source: 1-indexed line,
	/// 0-indexed column, matching [`SpanLookup`](crate::prelude::SpanLookup).
	pub fn line_col(&self, offset: usize) -> crate::prelude::LineCol {
		let prefix = &self.source[..offset];
		let line = prefix.bytes().filter(|byte| *byte == b'\n').count() as u32 + 1;
		let col = prefix.len() - prefix.rfind('\n').map(|nl| nl + 1).unwrap_or(0);
		crate::prelude::LineCol::new(line, col as u32)
	}

	/// Slice the source between two byte offsets recorded from [`Self::offset`].
	pub fn slice(&self, start: usize, end: usize) -> &'a str {
		&self.source[start..end]
	}

	/// Whether the cursor has reached the end of the source.
	pub fn is_eof(&self) -> bool { self.offset >= self.source.len() }

	/// The next character without consuming it.
	pub fn peek(&self) -> Option<char> { self.rest().chars().next() }

	/// Whether the remainder starts with `prefix`.
	pub fn starts_with(&self, prefix: &str) -> bool {
		self.rest().starts_with(prefix)
	}

	/// Consume and return the next character, if any.
	pub fn bump(&mut self) -> Option<char> {
		let ch = self.peek()?;
		self.offset += ch.len_utf8();
		Some(ch)
	}

	/// Consume `prefix` if the remainder starts with it, returning whether it did.
	pub fn eat(&mut self, prefix: &str) -> bool {
		if self.starts_with(prefix) {
			self.offset += prefix.len();
			true
		} else {
			false
		}
	}

	/// Consume leading ascii whitespace.
	pub fn skip_ws(&mut self) {
		while let Some(ch) = self.peek() {
			if ch.is_whitespace() {
				self.bump();
			} else {
				break;
			}
		}
	}

	/// Consume characters while `predicate` holds, returning the consumed slice.
	pub fn take_while(&mut self, mut predicate: impl FnMut(char) -> bool) -> &'a str {
		let start = self.offset;
		while let Some(ch) = self.peek() {
			if predicate(ch) {
				self.bump();
			} else {
				break;
			}
		}
		&self.source[start..self.offset]
	}

	/// Consume up to (but not including) `delimiter`, returning the consumed
	/// slice. Stops at end of input if the delimiter is never found.
	pub fn take_until(&mut self, delimiter: &str) -> &'a str {
		let start = self.offset;
		while !self.is_eof() && !self.starts_with(delimiter) {
			self.bump();
		}
		&self.source[start..self.offset]
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::prelude::*;

	#[beet_core::test]
	fn peek_and_bump() {
		let mut cursor = Cursor::new("ab");
		cursor.peek().unwrap().xpect_eq('a');
		cursor.bump().unwrap().xpect_eq('a');
		cursor.bump().unwrap().xpect_eq('b');
		cursor.is_eof().xpect_true();
	}

	#[beet_core::test]
	fn eat_and_take() {
		let mut cursor = Cursor::new("<div>hi</div>");
		cursor.eat("<").xpect_true();
		cursor
			.take_while(|ch| ch.is_alphanumeric())
			.xpect_eq("div");
		cursor.eat(">").xpect_true();
		cursor.take_until("</").xpect_eq("hi");
	}

	#[beet_core::test]
	fn skip_whitespace() {
		let mut cursor = Cursor::new("   x");
		cursor.skip_ws();
		cursor.peek().unwrap().xpect_eq('x');
	}
}
