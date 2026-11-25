//! [`PlainTextParser`], text as one entity holding it.
use crate::prelude::*;
use beet_core::prelude::*;

/// Reads text as one entity holding it as a [`Value::Str`]: plain text, and
/// any text type with no parser of its own, [`DocParsers`] falling back to
/// this for it.
///
/// Markdown read as text declares its [`Frontmatter`] beside it, so a route
/// scan reads a page's metadata with no content parser registered.
///
/// With [`plaintext_only`](Self::plaintext_only) only [`MediaType::Text`]
/// reads; by default any type where [`MediaType::is_text`] holds does.
#[derive(Debug, Default, Clone)]
pub struct PlainTextParser {
	/// Require an explicit [`MediaType::Text`].
	plaintext_only: bool,
}

impl PlainTextParser {
	/// Read any text type.
	pub fn new() -> Self { Self::default() }

	/// Require an explicit [`MediaType::Text`], refusing other text types
	/// like HTML or markdown.
	pub fn plaintext_only(mut self) -> Self {
		self.plaintext_only = true;
		self
	}
}

impl DocParser for PlainTextParser {
	fn media_types(&self) -> Vec<MediaType> { vec![MediaType::Text] }

	fn parse(
		&mut self,
		bytes: &MediaBytes,
		parsers: &DocParsers,
	) -> Result<ParsedDocument, ParseError> {
		let media_type = bytes.media_type();
		let accepted = match self.plaintext_only {
			true => *media_type == MediaType::Text,
			false => media_type.is_text(),
		};
		if !accepted {
			return Err(ParseError::UnsupportedType {
				unsupported: media_type.clone(),
				supported: vec![MediaType::Text],
			});
		}
		let root = Frontmatter::root_components(bytes, parsers)?;
		let text = Value::new(bytes.as_utf8()?.to_string());
		ParsedDocument::from(SceneBody::root(root.with_typed(&text)?)).xok()
	}
}

impl Installer for PlainTextParser {
	fn install(self, app: &mut App) { DocParsers::register_on(app, self); }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn plaintext_only_refuses_other_text() {
		PlainTextParser::new()
			.plaintext_only()
			.parse(&MediaBytes::new_css("a {}"), &DocParsers::default())
			.unwrap_err();
	}
}
