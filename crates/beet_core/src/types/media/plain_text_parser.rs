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
	/// The root declarations a text document carries, ie markdown's
	/// frontmatter, read by a route scan with no content parser registered.
	#[cfg(feature = "bsx")]
	declarations: Option<fn(&MediaBytes, &str) -> Result<RootDeclarations>>,
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

	/// Answer [`DocParser::declarations`] with `declarations`, ie a markdown
	/// frontmatter reader, so a route scan reads a page's metadata through
	/// the fallback.
	#[cfg(feature = "bsx")]
	pub fn with_declarations(
		mut self,
		declarations: fn(&MediaBytes, &str) -> Result<RootDeclarations>,
	) -> Self {
		self.declarations = Some(declarations);
		self
	}
}

impl DocParser for PlainTextParser {
	fn media_types(&self) -> Vec<MediaType> { vec![MediaType::Text] }

	fn parse(&mut self, bytes: &MediaBytes) -> Result<SceneBody, ParseError> {
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
		let text = Value::new(bytes.as_utf8()?.to_string());
		SceneBody::root(EntityBody::default().with_typed(&text)?).xok()
	}

	#[cfg(feature = "bsx")]
	fn declarations(
		&self,
		bytes: &MediaBytes,
		frontmatter_type: &str,
	) -> Result<RootDeclarations> {
		match self.declarations {
			Some(declarations) => declarations(bytes, frontmatter_type),
			None => RootDeclarations::default().xok(),
		}
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
			.parse(&MediaBytes::new_css("a {}"))
			.unwrap_err();
	}
}
