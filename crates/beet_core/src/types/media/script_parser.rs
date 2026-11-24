//! [`ScriptParser`], a javascript document as a script.
use crate::prelude::*;

/// Reads javascript as one entity carrying [`RunScript`] with its source as
/// its [`Value`], the shape every script takes.
#[derive(Debug, Default, Clone)]
pub struct ScriptParser;

impl DocParser for ScriptParser {
	fn media_types(&self) -> Vec<MediaType> { vec![MediaType::Javascript] }

	fn parse(
		&mut self,
		bytes: &MediaBytes,
		_parsers: &DocParsers,
	) -> Result<SceneBody, ParseError> {
		let source = Value::new(bytes.as_utf8()?.to_string());
		SceneBody::root(
			EntityBody::default()
				.with_typed(&RunScript)?
				.with_typed(&source)?,
		)
		.xok()
	}
}

impl Installer for ScriptParser {
	fn install(self, app: &mut App) { DocParsers::register_on(app, self); }
}
