//! The `beet_ui` BSX front-end: a thin [`NodeParser`] that delegates parsing to
//! the core BSX parser ([`beet_core::bsx`]).
//!
//! The grammar, the value model, the `bx:` directives, the event/verb seam, and
//! the AST-to-world resolution all live in `beet_core`; this file only adapts
//! that to the ui-side [`MediaParser`] dispatch. One parser, one grammar:
//! [`MediaType::Bsx`] parses with BSX features on, [`MediaType::Html`] with them
//! off (the HTML-only subset). The default event/verb registration lives in
//! [`BsxDefaultsPlugin`].
mod defaults;
pub use defaults::*;

use crate::prelude::*;
use beet_core::prelude::*;

/// A [`NodeParser`] for the BSX and HTML media types, delegating to the core
/// parser.
///
/// The parsed tree is resolved into the calling entity through
/// `insert_template`, so slots, references, and the lifecycle resolve in the one
/// instantiation path.
#[derive(Debug, Clone, Default)]
pub struct BsxParser {
	/// Tokenization configuration (which grammar features are enabled).
	pub config: BsxParseConfig,
}

impl BsxParser {
	/// A parser accepting the full BSX grammar.
	pub fn bsx() -> Self {
		Self {
			config: BsxParseConfig::bsx(),
		}
	}

	/// A parser accepting only HTML (BSX features disabled).
	pub fn html() -> Self {
		Self {
			config: BsxParseConfig::html(),
		}
	}

	/// Parse `text` and build the result into `entity`.
	fn parse_into(&self, entity: &mut EntityWorldMut, text: &str) -> Result {
		let nodes = parse_document(text, &self.config)?;
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

impl NodeParser for BsxParser {
	fn parse(&mut self, cx: ParseContext) -> Result<(), ParseError> {
		let media_type = cx.bytes.media_type().clone();
		let config = match media_type {
			MediaType::Bsx => BsxParseConfig::bsx(),
			MediaType::Html => BsxParseConfig::html(),
			other => {
				return Err(ParseError::UnsupportedType {
					unsupported: other,
					supported: vec![MediaType::Bsx, MediaType::Html],
				});
			}
		};
		let parser = BsxParser { config };
		let text = cx.bytes.as_utf8().map_err(BevyError::from)?;
		parser.parse_into(cx.entity, text)?;
		Ok(())
	}
}
