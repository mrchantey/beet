use crate::prelude::*;
use beet_core::prelude::*;

/// Parses raw bytes as UTF-8 text and stores the result as a [`Value::Str`] component.
///
/// On repeated calls the value is only updated when the content has changed,
/// avoiding unnecessary change-detection triggers.
///
/// If a `path` is provided a [`FileSpan`] covering the entire text is inserted
/// alongside the value.
#[derive(Debug, Default, Clone)]
pub struct PlainTextParser;

impl PlainTextParser {
	pub fn new() -> Self { Self::default() }
}

impl NodeParser for PlainTextParser {
	fn parse(
		&mut self,
		cx: ParseContext,
	) -> Result<(), ParseError> {
		let media_type = cx.bytes.media_type();
		if !media_type.is_text() && *media_type != MediaType::Bytes {
			return Err(ParseError::UnsupportedType {
				unsupported: media_type.clone(),
				supported: vec![MediaType::Text],
			});
		}

		let text = core::str::from_utf8(cx.bytes.bytes())
			.map_err(|err| ParseError::Other(bevyhow!("invalid utf-8: {err}")))?
			.to_string();

		let span = cx.path.map(|path| {
			let mut tracker = SpanTracker::new(path);
			tracker.extend(&text);
			tracker.into_full_span()
		});

		cx.entity.set_if_ne_or_insert(Value::new(text));
		if let Some(span) = span {
			cx.entity.set_if_ne_or_insert(span);
		}

		Ok(())
	}
}
