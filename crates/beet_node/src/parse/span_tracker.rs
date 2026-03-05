use beet_core::prelude::*;
use std::sync::Arc;

/// Pre-indexed line boundary lookup for random-access byte-offset to [`LineCol`]
/// conversion.
///
/// Built once from the full input text, then used to convert any byte offset
/// (or borrowed `&str` slice) into a [`LineCol`] or [`FileSpan`].
///
/// ## Example
/// ```rust
/// # use beet_node::prelude::*;
/// # use beet_core::prelude::*;
/// let input = "hello\nworld\nfoo";
/// let lookup = SpanLookup::new(input, WsPathBuf::new("test.txt"));
/// lookup.line_col(0).xpect_eq(LineCol::new(1, 0));
/// lookup.line_col(6).xpect_eq(LineCol::new(2, 0));
/// lookup.line_col(8).xpect_eq(LineCol::new(2, 2));
/// ```
#[derive(Debug, Clone)]
pub struct SpanLookup {
	/// Byte offset of each line start. `line_starts[0]` is always 0.
	line_starts: Vec<usize>,
	/// The full input length in bytes.
	input_len: usize,
	/// Pointer to the start of the input string (for slice offset calculation).
	input_ptr: usize,
	/// Shared path used for all spans produced by this lookup.
	path: Arc<WsPathBuf>,
}

impl SpanLookup {
	/// Build a lookup from the full input text and file path.
	pub fn new(input: &str, path: WsPathBuf) -> Self {
		let mut line_starts = vec![0usize];
		for (idx, byte) in input.bytes().enumerate() {
			if byte == b'\n' {
				line_starts.push(idx + 1);
			}
		}
		Self {
			line_starts,
			input_len: input.len(),
			input_ptr: input.as_ptr() as usize,
			path: Arc::new(path),
		}
	}

	/// Convert a byte offset into a [`LineCol`].
	///
	/// Returns 1-indexed lines and 0-indexed columns consistent with
	/// [`LineCol`] conventions.
	///
	/// # Panics
	/// Panics if `offset` exceeds the input length.
	pub fn line_col(&self, offset: usize) -> LineCol {
		assert!(
			offset <= self.input_len,
			"byte offset {offset} exceeds input length {}",
			self.input_len
		);
		// binary search for the line containing this offset
		let line_idx = match self.line_starts.binary_search(&offset) {
			Ok(exact) => exact,
			Err(insert) => insert - 1,
		};
		let col = offset - self.line_starts[line_idx];
		// LineCol uses 1-indexed lines
		LineCol::new((line_idx + 1) as u32, col as u32)
	}

	/// Compute the byte offset of a borrowed `&str` slice relative to the
	/// original input.
	///
	/// # Panics
	/// Panics if the slice does not point within the original input.
	pub fn slice_offset(&self, slice: &str) -> usize {
		let slice_ptr = slice.as_ptr() as usize;
		let offset = slice_ptr
			.checked_sub(self.input_ptr)
			.expect("slice does not point within the original input");
		assert!(
			offset <= self.input_len,
			"slice offset {offset} exceeds input length {}",
			self.input_len
		);
		offset
	}

	/// Build a [`FileSpan`] covering a borrowed `&str` slice.
	pub fn span_of(&self, slice: &str) -> FileSpan {
		let start_offset = self.slice_offset(slice);
		let end_offset = start_offset + slice.len();
		FileSpan::new(
			self.path.as_ref().clone(),
			self.line_col(start_offset),
			self.line_col(end_offset),
		)
	}

	/// Build a [`FileSpan`] covering the entire input.
	pub fn full_span(&self) -> FileSpan {
		FileSpan::new(
			self.path.as_ref().clone(),
			LineCol::new(1, 0),
			self.line_col(self.input_len),
		)
	}

	/// Returns the file path.
	pub fn path(&self) -> &WsPathBuf { &self.path }
}

/// Tracks the current position within a text file as content is fed through it,
/// enabling [`FileSpan`] construction at any point during streaming.
///
/// Positions follow the same conventions as [`LineCol`]: 1-indexed lines and
/// 0-indexed columns.
///
/// ## Example
/// ```rust
/// # use beet_node::prelude::*;
/// # use beet_core::prelude::*;
/// let mut tracker = SpanTracker::new(WsPathBuf::new("foo.txt"));
/// let start = tracker.pos();
/// tracker.extend("hello\nworld");
/// let span = tracker.span_from(start);
/// assert_eq!(span.start(), LineCol::new(1, 0));
/// assert_eq!(span.end(), LineCol::new(2, 5));
/// ```
#[derive(Debug, Clone)]
pub struct SpanTracker {
	path: WsPathBuf,
	current: LineCol,
}

impl SpanTracker {
	/// Create a new tracker anchored to the given file path, starting at `1:0`.
	pub fn new(path: WsPathBuf) -> Self {
		Self {
			path,
			current: LineCol::default(),
		}
	}

	/// Returns the current position without advancing it.
	pub fn pos(&self) -> LineCol { self.current }

	/// Returns the file path.
	pub fn path(&self) -> &WsPathBuf { &self.path }

	/// Advance the tracked position by scanning `text`.
	///
	/// Each `\n` increments the line and resets the column to 0.
	/// Any other character increments the column.
	pub fn extend(&mut self, text: &str) {
		for ch in text.chars() {
			if ch == '\n' {
				self.current.line += 1;
				self.current.col = 0;
			} else {
				self.current.col += 1;
			}
		}
	}

	/// Build a [`FileSpan`] from `start` to the current position.
	pub fn span_from(&self, start: LineCol) -> FileSpan {
		FileSpan::new(self.path.clone(), start, self.current)
	}

	/// Build a [`FileSpan`] covering the entire content fed so far,
	/// ie from the default start position (`1:0`) to the current position.
	pub fn into_full_span(self) -> FileSpan {
		FileSpan::new(self.path, LineCol::default(), self.current)
	}
}


#[cfg(test)]
mod test {
	use super::*;

	// -- SpanLookup tests --

	#[test]
	fn lookup_single_line() {
		let input = "hello";
		let lookup = SpanLookup::new(input, WsPathBuf::new("test.txt"));
		lookup.line_col(0).xpect_eq(LineCol::new(1, 0));
		lookup.line_col(3).xpect_eq(LineCol::new(1, 3));
		lookup.line_col(5).xpect_eq(LineCol::new(1, 5));
	}

	#[test]
	fn lookup_multi_line() {
		let input = "hello\nworld\nfoo";
		let lookup = SpanLookup::new(input, WsPathBuf::new("test.txt"));
		lookup.line_col(0).xpect_eq(LineCol::new(1, 0));
		lookup.line_col(5).xpect_eq(LineCol::new(1, 5)); // the \n char
		lookup.line_col(6).xpect_eq(LineCol::new(2, 0)); // 'w'
		lookup.line_col(11).xpect_eq(LineCol::new(2, 5)); // the second \n
		lookup.line_col(12).xpect_eq(LineCol::new(3, 0)); // 'f'
		lookup.line_col(15).xpect_eq(LineCol::new(3, 3)); // end
	}

	#[test]
	fn lookup_trailing_newline() {
		let input = "a\nb\n";
		let lookup = SpanLookup::new(input, WsPathBuf::new("test.txt"));
		lookup.line_col(4).xpect_eq(LineCol::new(3, 0));
	}

	#[test]
	fn lookup_span_of_slice() {
		let input = "hello\nworld";
		let lookup = SpanLookup::new(input, WsPathBuf::new("test.txt"));
		// "world" starts at byte 6
		let slice = &input[6..11];
		let span = lookup.span_of(slice);
		span.start().xpect_eq(LineCol::new(2, 0));
		span.end().xpect_eq(LineCol::new(2, 5));
		span.path().xpect_eq(WsPathBuf::new("test.txt"));
	}

	#[test]
	fn lookup_full_span() {
		let input = "line1\nline2";
		let lookup = SpanLookup::new(input, WsPathBuf::new("test.txt"));
		let span = lookup.full_span();
		span.start().xpect_eq(LineCol::new(1, 0));
		span.end().xpect_eq(LineCol::new(2, 5));
	}

	#[test]
	fn lookup_empty_input() {
		let input = "";
		let lookup = SpanLookup::new(input, WsPathBuf::new("test.txt"));
		lookup.line_col(0).xpect_eq(LineCol::new(1, 0));
	}

	// -- SpanTracker tests --

	#[test]
	fn empty_text_stays_at_default() {
		let mut tracker = SpanTracker::new(WsPathBuf::new("test.txt"));
		tracker.extend("");
		tracker.pos().xpect_eq(LineCol::default());
	}

	#[test]
	fn single_line_no_newline() {
		let mut tracker = SpanTracker::new(WsPathBuf::new("test.txt"));
		tracker.extend("hello");
		tracker.pos().xpect_eq(LineCol::new(1, 5));
	}

	#[test]
	fn newline_increments_line_resets_col() {
		let mut tracker = SpanTracker::new(WsPathBuf::new("test.txt"));
		tracker.extend("hello\nworld");
		tracker.pos().xpect_eq(LineCol::new(2, 5));
	}

	#[test]
	fn multiple_lines() {
		let mut tracker = SpanTracker::new(WsPathBuf::new("test.txt"));
		tracker.extend("a\nb\nc");
		tracker.pos().xpect_eq(LineCol::new(3, 1));
	}

	#[test]
	fn span_from_captures_range() {
		let mut tracker = SpanTracker::new(WsPathBuf::new("test.txt"));
		let start = tracker.pos(); // 1:0
		tracker.extend("hi\nthere");
		let span = tracker.span_from(start);
		span.start().xpect_eq(LineCol::new(1, 0));
		span.end().xpect_eq(LineCol::new(2, 5));
	}

	#[test]
	fn incremental_extend() {
		let mut tracker = SpanTracker::new(WsPathBuf::new("test.txt"));
		tracker.extend("foo");
		tracker.extend("\nbar");
		tracker.pos().xpect_eq(LineCol::new(2, 3));
	}

	#[test]
	fn into_full_span() {
		let tracker = {
			let mut t = SpanTracker::new(WsPathBuf::new("test.txt"));
			t.extend("line1\nline2\n");
			t
		};
		let span = tracker.into_full_span();
		span.start().xpect_eq(LineCol::default());
		span.end().xpect_eq(LineCol::new(3, 0));
	}
}
