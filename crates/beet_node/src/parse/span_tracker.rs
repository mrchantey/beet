use beet_core::prelude::*;

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
/// assert_eq!(span.start, LineCol::new(1, 0));
/// assert_eq!(span.end, LineCol::new(2, 5));
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
