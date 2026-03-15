use crate::prelude::*;
use beet_core::prelude::*;

/// Parses raw bytes as UTF-8 text and stores the result as a [`Value::Str`] component.
///
/// On repeated calls the value is only updated when the content has changed,
/// avoiding unnecessary change-detection triggers.
///
/// If a `path` is provided a [`FileSpan`] covering the entire text is inserted
/// alongside the value.
///
/// When `plaintext_only` is `true`, only [`MediaType::Text`] is accepted.
/// When `false` (the default), any media type where [`MediaType::is_text`]
/// returns `true` is allowed.
#[derive(Debug, Default, Clone)]
pub struct PlainTextParser {
	/// When `true`, require an explicit [`MediaType::Text`].
	/// When `false`, accept any text-based media type.
	plaintext_only: bool,
}

impl PlainTextParser {
	pub fn new() -> Self { Self::default() }

	/// Require an explicit [`MediaType::Text`], rejecting other text-based
	/// media type like HTML or Markdown.
	pub fn plaintext_only(mut self) -> Self {
		self.plaintext_only = true;
		self
	}
}

impl NodeParser for PlainTextParser {
	fn parse(&mut self, cx: ParseContext) -> Result<(), ParseError> {
		let media_type = cx.bytes.media_type();
		let accepted = if self.plaintext_only {
			*media_type == MediaType::Text
		} else {
			media_type.is_text() || *media_type == MediaType::Bytes
		};
		if !accepted {
			return Err(ParseError::UnsupportedType {
				unsupported: media_type.clone(),
				supported: vec![MediaType::Text],
			});
		}

		let text = cx.bytes.as_utf8()?;

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
