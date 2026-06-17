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
//! grammar: a `{literal}` lowers to a [`Value`], an `{@source:path}` binding to
//! its reactive sync components. The `bx:` directives (`bx:scope`, `bx:ref`,
//! `bx:click`, ...) attach their document-system and slot-marker components.
//!
//! ## Bindings
//!
//! All interpolation is reactive and explicitly source-prefixed:
//!
//! ```text
//! binding = "@doc"    ":" fieldpath init?   | "@prop" ":" fieldpath
//!         | "@res"    ":" ShortTypePath "." fieldpath
//!         | "@comp"   ":" ShortTypePath "." fieldpath
//!         | "@entity" ":" RefName "::" ShortTypePath "." fieldpath
//! fieldpath = a field path, eg count or user.name
//! init      = "=" literal             doc only, eg {@doc:count=0}
//! ```
//!
//! - `@doc:` the nearest ancestor user [`Document`], a [`FieldRef`]; the `=init`
//!   initializes a missing field
//! - `@res:` a reflected resource field, a [`ResourceFieldRef`]
//! - `@comp:` a reflected component field, a [`ReflectFieldRef`] targeting the
//!   current entity: the element in attribute position, the text node in text
//!   position
//! - `@entity:Name::` retargets a component binding to a `bx:ref` named entity;
//!   the names `BuildRoot`, `SnippetRoot`, `PageRoot` and `Router` are reserved
//!   ([`ReservedRef`]) for well-known entities (the latter two resolved lazily
//!   each sync pass) and may not be shadowed
//! - `@prop:` the enclosing `.bsx` template's props store, materialized from
//!   the caller's tag attributes (a binding-valued prop chains reactively)
//!
//! Bindings work in attribute position (`value=@res:Theme.contrast`), text
//! position (`{@doc:name}`), and as spread-tuple items
//! (`{(Slider{value:3}, @comp:Slider.value)}`), pairing a component insert with
//! a binding on the same entity. Event directives are a verb call:
//! `bx:click=increment{ field: @doc:count, amount: 3 }` runs the registered
//! `increment` verb with its named arguments against the host entity, the verb
//! writing the bound source directly ([`events`]). No mirror is lowered onto the
//! host: the verb mutates the real document/resource and document-sync fans the
//! change out to display bindings.
//!
//! ```
//! use beet_core::prelude::*;
//!
//! let mut world = (TemplatePlugin, DocumentPlugin).into_world();
//! let nodes = parse_document(
//!     r#"<section bx:scope="user"><p>{@doc:name="Ada"}</p><button bx:click=increment{ field: @doc:clicks }>+</button></section>"#,
//!     &BsxParseConfig::bsx(),
//! )
//! .unwrap();
//! let root = world
//!     .spawn_template(BsxTemplate::container(nodes, BsxTemplateRegistry::default()))
//!     .unwrap()
//!     .id();
//! world.update_local();
//!
//! // the `bx:scope` directive is a `DocumentScope`, so the text binding
//! // resolves `user.name`, initialized from the binding's `=init`
//! let section = world.entity(root).get::<Children>().unwrap()[0];
//! assert!(world.entity(section).contains::<DocumentScope>());
//! let paragraph = world.entity(section).get::<Children>().unwrap()[0];
//! let text = world.entity(paragraph).get::<Children>().unwrap()[0];
//! assert!(world.entity(text).contains::<FieldRef>());
//! assert_eq!(world.entity(text).get::<Value>().unwrap(), &Value::Str("Ada".into()));
//! // the event host carries no mirror: the verb writes the document directly
//! let button = world.entity(section).get::<Children>().unwrap()[1];
//! assert!(!world.entity(button).contains::<FieldRef>());
//! ```
//!
//! ## Templates
//!
//! An uppercase tag resolves by name: a Rust component/template via the type
//! registry, a `#[reflect(Resource)]` type (`<PackageConfig title=".."/>`
//! patches the live resource's named fields, producing no markup), or a
//! `<path::to::X>` `.bsx` template from the [`BsxTemplateRegistry`]. A `.bsx`
//! template's tag attributes materialize into its props store (bound in the
//! body via `@prop:`), and caller content composes through its `<Slot/>`:
//!
//! ```
//! use beet_core::prelude::*;
//!
//! let mut world = (TemplatePlugin, DocumentPlugin).into_world();
//! let mut registry = BsxTemplateRegistry::default();
//! registry
//!     .insert_source("Card", "<section><h2>{@prop:title}</h2><Slot/></section>")
//!     .unwrap();
//!
//! let nodes = parse_document(
//!     r#"<Card title="Intro"><p>hi</p></Card>"#,
//!     &BsxParseConfig::bsx(),
//! )
//! .unwrap();
//! let root =
//!     world.spawn_template(BsxTemplate::container(nodes, registry)).unwrap().id();
//! world.update_local();
//!
//! // `<Card>` built its `<section>` body into the entity, the caller's `<p>`
//! // routed into the slot, and `@prop:title` synced the caller's prop
//! let card = world.entity(root).get::<Children>().unwrap()[0];
//! assert_eq!(world.entity(card).get::<Element>().unwrap().tag(), "section");
//! let heading = world.entity(card).get::<Children>().unwrap()[0];
//! let title = world.entity(heading).get::<Children>().unwrap()[0];
//! assert_eq!(world.entity(title).get::<Value>().unwrap(), &Value::Str("Intro".into()));
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
mod style_resolver;
mod tag_resolver;
mod value;

pub use ast::*;
pub use events::*;
pub use parse::*;
pub use reflect::*;
pub use registry::*;
pub use style_resolver::*;
pub use tag_resolver::*;
pub use value::parse_value_expr_str;
#[cfg(feature = "bevy_async")]
pub use remote::*;
pub use resolve::*;
pub use schema::*;

use crate::prelude::*;

/// Registers the BSX event/verb seam resources so
/// `bx:<event>=verb{ arg: value, .. }` resolves at build time.
///
/// Both registries are **empty by default**: core knows no concrete event or
/// verb. An app (or `beet_ui`'s default registration) installs the concrete
/// `click` event installer and the example verb set.
pub struct BsxPlugin;

impl Plugin for BsxPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<EventRegistry>()
			.init_resource::<VerbRegistry>()
			.init_resource::<BsxTagResolvers>()
			.init_resource::<StyleResolver>()
			.init_resource::<BsxTemplateRegistry>();
	}
}
