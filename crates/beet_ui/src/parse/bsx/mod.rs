//! The `beet_ui` BSX front-end: a thin [`DocParser`] that delegates parsing to
//! the core BSX parser ([`beet_core::bsx`]).
//!
//! The grammar, the value model, the `bx:` directives, the event seam, and the
//! AST-to-world resolution all live in `beet_core`; this file only adapts that
//! to the ui-side [`DocParsers`] dispatch. One parser, one grammar:
//! [`MediaType::Bsx`] parses with BSX features on, [`MediaType::Html`] with them
//! off (the HTML-only subset), and [`MediaType::Xml`] as XML, building plain
//! markup that writes back as it was read. The default event registration lives in
//! [`BsxDefaultsPlugin`], and the `bx:<event>` script installer it wires in
//! `event_script`.
mod defaults;
pub use defaults::*;
#[cfg(feature = "scripting")]
mod event_script;

use crate::prelude::*;
use beet_core::prelude::*;

/// A [`DocParser`] for the BSX, HTML and XML media types, delegating to the
/// core parser in the dialect the media type names
/// ([`BsxParseConfig::from_media_type`]).
///
/// The parsed tree is resolved into the calling entity through
/// `insert_template`, so slots, references, and the lifecycle resolve in the one
/// instantiation path.
#[derive(Debug, Clone, Default)]
pub struct BsxParser;

impl BsxParser {
	/// Parse `text` and build the result into `entity`.
	fn parse_into(
		config: &BsxParseConfig,
		entity: &mut EntityWorldMut,
		text: &str,
	) -> Result {
		let nodes = BsxNode::parse_document(text, config)?;
		// xml resolves nothing: every element is markup as written
		if config.is_xml() {
			BsxNode::spawn_markup(&nodes, entity);
			return Ok(());
		}
		// snapshot the BSX-template registry so `<path::to::X>` resolves mid-build.
		let registry = entity
			.world_scope(|world| {
				world.get_resource::<BsxTemplateRegistry>().cloned()
			})
			.unwrap_or_default();
		// a build failure (eg a schema violation) rides `TemplateError` on the
		// entity rather than aborting the parse, so the returned error is dropped.
		let _ = entity.insert_template(BsxTemplate::container(nodes, registry));
		Ok(())
	}
}

impl DocParser for BsxParser {
	fn media_types(&self) -> Vec<MediaType> {
		BsxParseConfig::MEDIA_TYPES.to_vec()
	}

	fn declarations(
		&self,
		bytes: &MediaBytes,
		_frontmatter_type: &str,
	) -> Result<RootDeclarations> {
		let config = BsxParseConfig::from_media_type(bytes.media_type())
			.unwrap_or_default();
		BsxNode::parse_document(core::str::from_utf8(bytes)?, &config)?
			.xmap(|nodes| RootDeclarations::from_bsx(&nodes))
			.xok()
	}

	fn parse(&mut self, cx: ParseContext) -> Result<(), ParseError> {
		let media_type = cx.bytes.media_type();
		let config =
			BsxParseConfig::from_media_type(media_type).ok_or_else(|| {
				ParseError::UnsupportedType {
					unsupported: media_type.clone(),
					supported: BsxParseConfig::MEDIA_TYPES.to_vec(),
				}
			})?;
		let text = cx.bytes.as_utf8().map_err(BevyError::from)?;
		Self::parse_into(&config, cx.entity, text)?;
		Ok(())
	}
}

#[cfg(feature = "bsx")]
impl Installer for BsxParser {
	fn install(self, app: &mut App) { DocParsers::register_on(app, self); }
}
