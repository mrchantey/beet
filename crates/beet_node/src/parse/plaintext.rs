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
		entity: AsyncEntity,
		bytes: Vec<u8>,
		path: Option<WsPathBuf>,
	) -> impl Future<Output = Result> {
		async move {
			let text = std::str::from_utf8(&bytes)
				.map_err(|e| bevyhow!("invalid utf-8: {e}"))?
				.to_string();

			let span = path.map(|p| {
				let mut tracker = SpanTracker::new(p);
				tracker.extend(&text);
				tracker.into_full_span()
			});

			entity
				.with_then(move |mut entity| {
					entity.set_if_ne_or_insert(Value::new(text));
					if let Some(span) = span {
						entity.set_if_ne_or_insert(span);
					}
					Ok(())
				})
				.await
		}
	}
}
