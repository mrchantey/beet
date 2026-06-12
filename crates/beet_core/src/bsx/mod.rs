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
//!
//! # Syntax
//!
//! Lowercase tags are HTML [`Element`]s; attributes become attribute child
//! entities; text becomes a [`Value`]. A braced `{..}` block is the value
//! grammar: a `{#field}` reference lowers to a document-bound [`FieldRef`], a
//! `{literal}` to a [`Value`]. The `bx:` directives (`bx:scope`, `bx:for`,
//! `bx:click`, ...) attach their document-system and slot-marker components:
//!
//! ```
//! use beet_core::prelude::*;
//!
//! let mut world = TemplatePlugin::world();
//! let nodes = parse_document(
//!     r#"<section bx:scope="user"><h1>Hello</h1><p>{#name}</p></section>"#,
//!     &BsxParseConfig::bsx(),
//! )
//! .unwrap();
//! let root = world
//!     .spawn_template(BsxTemplate::container(nodes, BsxTemplateRegistry::default()))
//!     .unwrap()
//!     .id();
//!
//! // the lowercase tag is an `Element`, the `bx:scope` directive a `DocumentScope`
//! let section = world.entity(root).get::<Children>().unwrap()[0];
//! assert_eq!(world.entity(section).get::<Element>().unwrap().tag(), "section");
//! assert!(world.entity(section).contains::<DocumentScope>());
//! // the `{#name}` block bound the paragraph's text to a document field
//! let paragraph = world.entity(section).get::<Children>().unwrap()[1];
//! let text = world.entity(paragraph).get::<Children>().unwrap()[0];
//! assert!(world.entity(text).contains::<FieldRef>());
//! ```
//!
//! An uppercase tag resolves by name: a Rust component/template via the type
//! registry, or a `<path::to::X>` `.bsx` template from the
//! [`BsxTemplateRegistry`], which composes caller content through its `<Slot/>`:
//!
//! ```
//! use beet_core::prelude::*;
//!
//! let mut world = TemplatePlugin::world();
//! let mut registry = BsxTemplateRegistry::default();
//! registry.insert_source("Card", "<section><Slot/></section>").unwrap();
//!
//! let nodes =
//!     parse_document("<Card><p>hi</p></Card>", &BsxParseConfig::bsx()).unwrap();
//! let root =
//!     world.spawn_template(BsxTemplate::container(nodes, registry)).unwrap().id();
//!
//! // `<Card>` built its `<section>` body into the entity, the caller's `<p>`
//! // routed into the slot
//! let card = world.entity(root).get::<Children>().unwrap()[0];
//! assert_eq!(world.entity(card).get::<Element>().unwrap().tag(), "section");
//! ```
//!
//! The remaining surface (bare-position component spreads `{(A, B)}`, the `$ref`
//! entity references, enum/struct/list literals) lives in the value grammar
//! ([`value`]) and AST-to-world resolution ([`resolve`]).

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
