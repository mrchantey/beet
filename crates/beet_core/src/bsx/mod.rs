//! The BSX parser: one hand-written recursive-descent cursor parser whose
//! disabled-surface configuration is exactly HTML.
//!
//! BSX is the full grammar (uppercase resolution, the value grammar, `bx:`
//! directives); HTML is BSX with that surface disabled. The parser is
//! XML-inspired markup, not "an HTML subset": one grammar, with HTML the markup
//! it accepts when the extra surface is switched off. It builds a
//! [`BsxNode`](ast::BsxNode) syntax tree, resolved into a document-wired entity
//! tree through the template substrate by [`BsxTemplate`], so a `.bsx` file
//! produces trees identical to what `rsx!` lowers to.
//!
//! Author + parse + build live here in `beet_core`; rendering the built tree to
//! HTML or charcell lives in `beet_ui`. The `MediaParser`/`MediaRenderer`
//! dispatch also stays in `beet_ui`, delegating BSX parsing to
//! [`parse_document`] + [`BsxTemplate`].
//!
//! Internal split: the syntax tree ([`ast`]), the cursor
//! ([`cursor`]), the markup parser ([`parse`]), the value grammar ([`value`]),
//! literal-to-reflect resolution ([`reflect`]), AST-to-world resolution
//! ([`resolve`]), the event/verb seam ([`events`]), and the BSX-template
//! registry ([`registry`]).

mod ast;
mod cursor;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
mod entry;
mod events;
mod parse;
mod reflect;
mod registry;
#[cfg(feature = "bevy_async")]
mod remote;
mod resolve;
mod schema;
mod value;

pub use ast::*;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
pub use entry::*;
pub use events::*;
pub use parse::*;
pub use registry::*;
pub use value::parse_value_expr_str;
#[cfg(feature = "bevy_async")]
pub use remote::*;
pub use resolve::*;
pub use schema::*;

use crate::prelude::*;

/// Registers the BSX event/verb seam resources so `bx:<event>=<verb>#field`
/// resolves at build time.
///
/// Both registries are **empty by default**: core knows no concrete event or
/// verb. An app (or `beet_ui`'s default registration) installs the concrete
/// `click` event installer and the example verb set. This plugin only seeds the
/// empty registries plus the named-handler escape hatch.
pub struct BsxPlugin;

impl Plugin for BsxPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<EventRegistry>()
			.init_resource::<VerbRegistry>()
			.init_resource::<BsxHandlerRegistry>()
			.init_resource::<BsxTemplateRegistry>();
	}
}
