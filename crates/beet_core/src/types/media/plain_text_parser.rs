//! [`PlainTextParser`], text as one entity holding it.
use crate::prelude::*;

/// Reads text as one entity holding it as a [`Value::Str`]: plain text, and
/// any text type with no parser of its own, [`DocParsers`] falling back to
/// this for it.
///
/// With [`plaintext_only`](Self::plaintext_only) only [`MediaType::Text`]
/// reads; by default any type where [`MediaType::is_text`] holds does.
#[derive(Debug, Default, Clone)]
pub struct PlainTextParser {
	/// Require an explicit [`MediaType::Text`].
	plaintext_only: bool,
	/// The components a text document declares at its root, ie markdown's
	/// frontmatter, read with no content parser registered.
	root_components: Option<RootComponentsFn>,
}

/// Reads the components a text document declares at its root.
type RootComponentsFn = fn(&MediaBytes, &DocParsers) -> Result<EntityBody>;

impl PlainTextParser {
	/// Read any text type.
	pub fn new() -> Self { Self::default() }

	/// Require an explicit [`MediaType::Text`], refusing other text types
	/// like HTML or markdown.
	pub fn plaintext_only(mut self) -> Self {
		self.plaintext_only = true;
		self
	}

	/// Declare what `root_components` reads beside the text, ie a markdown
	/// frontmatter reader, so a route scan reads a page's metadata through
	/// the fallback.
	pub fn with_root_components(
		mut self,
		root_components: RootComponentsFn,
	) -> Self {
		self.root_components = Some(root_components);
		self
	}
}

impl DocParser for PlainTextParser {
	fn media_types(&self) -> Vec<MediaType> { vec![MediaType::Text] }

	fn parse(
		&mut self,
		bytes: &MediaBytes,
		parsers: &DocParsers,
	) -> Result<SceneBody, ParseError> {
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
		let root = match self.root_components {
			Some(root_components) => root_components(bytes, parsers)?,
			None => EntityBody::default(),
		};
		let text = Value::new(bytes.as_utf8()?.to_string());
		SceneBody::root(root.with_typed(&text)?).xok()
	}
}

impl Installer for PlainTextParser {
	fn install(self, app: &mut App) { DocParsers::register_on(app, self); }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn plaintext_only_refuses_other_text() {
		PlainTextParser::new()
			.plaintext_only()
			.parse(&MediaBytes::new_css("a {}"), &DocParsers::default())
			.unwrap_err();
	}
}
