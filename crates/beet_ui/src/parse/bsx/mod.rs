//! The BSX parser: one hand-written recursive-descent cursor parser whose
//! features-off configuration is exactly HTML.
//!
//! BSX is the full grammar (uppercase resolution, the value grammar, `bx:`
//! directives); HTML is BSX with those features disabled, gated behind the `bsx`
//! cargo feature so the HTML-only mode is a real, tested configuration. The
//! parser builds a [`BsxNode`](ast::BsxNode) syntax tree, resolved into a
//! document-wired entity tree through the frozen template substrate by
//! [`BsxTemplate`], so a `.bsx` file produces trees identical to what `rsx!`
//! lowers to.
//!
//! Internal split (`bsx_parser.md`): the syntax tree ([`ast`]), the cursor
//! ([`cursor`]), the markup parser ([`parse`]), the value grammar ([`value`]),
//! literal-to-reflect resolution ([`reflect`]), AST-to-world resolution
//! ([`resolve`]), the event vocabulary ([`events`]), and the BSX-template
//! registry ([`registry`]).

mod ast;
mod cursor;
mod events;
mod parse;
mod reflect;
mod registry;
mod remote;
mod resolve;
mod schema;
mod value;

pub use ast::*;
pub use events::*;
pub use parse::*;
pub use registry::*;
pub use remote::*;
pub use resolve::*;
pub use schema::*;

use crate::prelude::*;
use beet_core::prelude::*;

/// A [`NodeParser`] for the BSX and HTML media types.
///
/// One parser, one grammar: [`MediaType::Bsx`] dispatches with BSX features on,
/// [`MediaType::Html`] with them off (the HTML-only subset). The parsed tree is
/// resolved into the calling entity through `insert_template`, so slots,
/// references, and the lifecycle resolve in the one instantiation path.
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
		entity.insert_template(BsxTemplate::container(nodes, registry));
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
