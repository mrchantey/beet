//! The BSX parser: one hand-written recursive-descent cursor parser whose
//! disabled-surface configuration is exactly HTML.
//!
//! BSX is the full grammar (uppercase resolution, the value grammar, `bx:`
//! directives); HTML is BSX with that surface disabled. The parser is
//! XML-inspired markup, not "an HTML subset": one grammar, with HTML the markup
//! it accepts when the extra surface is switched off. It builds a
//! [`BsxNode`] syntax tree, resolved into a document-wired entity
//! tree through the template substrate by [`BsxTemplate`], so a `.bsx` file
//! produces trees identical to what `rsx!` lowers to.
//!
//! Author + parse + build live here in `beet_core`, beside the format registry
//! [`DocParsers`]; rendering the built tree to HTML or charcell lives in
//! `beet_ui`, as do its BSX parser, which delegates to
//! [`BsxNode::parse_document`] + [`BsxTemplate`], and the `DocRenderers`
//! registry.
//!
//! Internal split, one directory per stage: source text to syntax tree in
//! `parse` (the cursor, the markup parser, the value grammar, the
//! source-format registry), syntax tree to explicit syntax tree in
//! `directive` (every `bx:` directive rewritten away, touching no world),
//! literal to reflected value in `reflect` (with the root declaration block
//! and prop-schema verification), and syntax tree to world in `resolve` (the
//! build walk and the resolver hooks a host installs into it). Alongside them sit the BSX-template registry
//! ([`BsxTemplateRegistry`]) and the entry-document front-end.
//!
//! # Syntax
//!
//! Lowercase tags are HTML [`Element`]s; attributes become attribute child
//! entities; text becomes a [`Value`]. A braced `{..}` block is the value
//! grammar: a `{literal}` lowers to a [`Value`], an `{@source:path}` binding to
//! its reactive sync components. `<script>` and `<style>` read their body as
//! raw text, HTML's own rule. An uppercase tag or a spread nothing is
//! registered under is not an error:
//! it warns, is held with its value in the entity's [`Unregistered`] and, for
//! a tag, builds its children anyway, so a lean binary loads the same document
//! shape a full one does and only its behavior is missing.
//!
//! [`bx:cfg`](BuildCondition) is the one directive that breaks that last
//! sentence, deliberately: it removes its node and subtree from the syntax tree
//! before the build walk, so an excluded branch has no shape to load. Structure
//! stays universal by default because an inert entity costs nothing; a node that
//! performs a build-time EFFECT (`<Template src>` reading a file) has no inert
//! form, and `bx:cfg` is how such a branch is kept out of a build that must not
//! run it. It leaves a [`CfgExcluded`] tombstone,
//! so an excluded route still reports why it is missing rather than 404ing.
//! Asserting the same condition instead of applying it is
//! [`RequireCfg`](crate::prelude::RequireCfg), which refuses the whole load.
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
//!   (`ReservedRef`) for well-known entities (the latter two resolved lazily
//!   each sync pass) and may not be shadowed
//! - `@prop:` the enclosing `.bsx` template's props store, materialized from
//!   the caller's tag attributes (a binding-valued prop chains reactively)
//!
//! Bindings work in attribute position (`value=@res:Theme.contrast`), text
//! position (`{@doc:name}`), and as spread-tuple items
//! (`{(Slider{value:3}, @comp:Slider.value)}`), pairing a component insert with
//! a binding on the same entity.
//!
//! ## HTML's words and beet's
//!
//! Beet's powers ride HTML's own vocabulary or a component, never a parallel
//! tag, and one rule tells the two apart: a bare attribute or tag means what
//! HTML means by it, and `bx:` marks beet's own meaning. A prefab is
//! `<template>`, an element carrying [`Prefab`] whose subtree is held inert
//! as the item a [`ReactiveChildren`] clones per row. A slot is HTML's
//! `<slot>` and `slot="name"` (`<Slot>` is refused with the spelling to use):
//! a child's `slot` routes it into a `<slot>` in its parent's subtree, and one
//! whose parent offers no slots at all stays HTML's attribute. A script beet
//! runs is `<script bx:run>`, run by beet rather than by the browser on load,
//! under the grant a sibling spread gives it (`beet_action`'s
//! `ScriptConfig`); a bare `<script>` stays page content the HTML renderer
//! emits for the browser. An event handler is `<script bx:event="click">`, a
//! child of the element it answers for, which the plugin owning the event
//! runs with the event target bound to `target`: no mirror is lowered onto the
//! host, the script writing the real document through the same
//! ancestor-document walk a display binding reads through.
//!
//! ## Directives
//!
//! A `bx:` directive is sugar for explicit BSX and nothing else, rewritten
//! between parsing and the build ([`BsxDirectives`]), so the scene, a save and
//! the runtime never learn it existed. The line between the two kinds of
//! shorthand is the world: a directive needs none and runs as a document
//! lowers, where a `#[template]` needs one and runs at spawn. A rewrite's
//! output is rewritten in turn, so sugar may stand on sugar. The standard
//! set:
//!
//! ```text
//! bx:scope="user"                    {DocumentScope{path:"user"}}
//! bx:for="entries" bx:key="name"     {(ReactiveChildren{key:"name"}, @doc:entries)}
//!                                    with the children wrapped in a <template>
//! bx:style="decls"                   {InlineStyle{declarations:"decls"}}
//! <script bx:run>                    <script {RunScript}>
//! <script bx:event="click">          <script {OnEvent{event:"click"}}>
//! bx:onclick="src"                   a child <script bx:event="click">src</script>,
//!                                    one `bx:on<event>` directive for every event
//! ```
//!
//! A downstream directive is a [`BsxDirective`] installed like any registry
//! entry, `app.install(MyDirective)`, landing in the set [`DocParsers`]
//! carries and shadowing an earlier one for the keys it matches. A `bx:` key
//! nothing matches refuses the document. `bx:ref`, `bx:cfg` and `bx:schema`
//! are still read by the build walk itself.
//!
//! ```
//! use beet_core::prelude::*;
//!
//! let mut world = (TemplatePlugin, DocumentPlugin).into_world();
//! let nodes = BsxNode::parse_document(
//!     r#"<section bx:scope="user"><p>{@doc:name="Ada"}</p><button bx:onclick="target.with_field('clicks', clicks => clicks + 1)">+</button></section>"#,
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
//! // the event host carries no mirror: the script writes the document directly
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
//! body via `@prop:`), caller content composes through its `<slot/>`, and the
//! entity it builds into records its key as a [`TemplateInstance`]:
//!
//! ```
//! use beet_core::prelude::*;
//!
//! let mut world = (TemplatePlugin, DocumentPlugin).into_world();
//! let mut registry = BsxTemplateRegistry::default();
//! registry
//!     .insert_source("Card", "<section><h2>{@prop:title}</h2><slot/></section>")
//!     .unwrap();
//!
//! let nodes = BsxNode::parse_document(
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
//! assert_eq!(world.entity(card).get::<TemplateInstance>().unwrap().0, vec![SmolStr::from("Card")]);
//! let heading = world.entity(card).get::<Children>().unwrap()[0];
//! let title = world.entity(heading).get::<Children>().unwrap()[0];
//! assert_eq!(world.entity(title).get::<Value>().unwrap(), &Value::Str("Intro".into()));
//! ```
//!
//! A template's root element is the tag's own entity (comments beside it
//! document the file and build nothing, as beside an entry's root), so an
//! attribute beyond its declared props (a Rust template's fields, a `.bsx` template's
//! `bx:schema`) falls through to that element: `<Link href="/docs" hidden>`
//! renders `<a href="/docs" hidden class="btn ..">`. A `class` joins the
//! element's own, any other key replaces its value, and a template with no
//! single root element refuses the attribute rather than dropping it. A `.bsx`
//! template declaring no schema, like `<Card>` above, takes every attribute as
//! a prop.
//!
//! The remaining surface (bare-position component spreads `{(A, B)}`, the `$ref`
//! entity references, enum/struct/list literals) lives in the value grammar
//! (`parse`) and AST-to-world resolution (`resolve`).
//!
//! [`Document`]: crate::prelude::Document
//! [`Element`]: crate::prelude::Element
//! [`FieldRef`]: crate::prelude::FieldRef
//! [`ReflectFieldRef`]: crate::prelude::ReflectFieldRef
//! [`ResourceFieldRef`]: crate::prelude::ResourceFieldRef
//! [`Unregistered`]: crate::prelude::Unregistered
//! [`Value`]: crate::prelude::Value
//!
//! # Authoring
//!
//! Every entity is authored under the tag of the type it most *is*. `<div>` and
//! `<span>` mean "this paints as a box of text" and are never a carrier for the
//! real type: a behavior loop is `<Repeat>`, a thread is `<Thread>`, a route is
//! `<Route>`, and whatever is left over rides a `{spread}` on that same entity.
//! Bad: `<div {(Route{path:"deploy"}, ExchangeSequence)}>`; good:
//! `<Route path="deploy" {ExchangeSequence}>`. Between co-located types the
//! entity's action wins (one action per entity, see `beet_action`):
//! `<Repeat {RunThread}>`, not `<RunThread {Repeat}>`; absent an action, the
//! noun the entity names wins: `<Thread {(Sequence, FsStore{path:".."})}>`.
//!
//! A generic type resolves by base name to its sole registered instantiation,
//! as a tag exactly as in a spread, so `<Repeat>`, `<Sequence>` and
//! `<RepeatTimes total_times=2>` all author directly. For a plain grouping use
//! `<Fragment>`, not `<div>`: it carries spreads, directives and children but
//! emits no element (`<Template>` is the *include* front-end,
//! `<Template src="..">`; with no `src` a directives-only no-op).
//!
//! `<Tag/>` resolves a component or template by short type path and spawns its
//! own entity; `{Spread}` / `{(A, B)}` adds components to the *current* entity.
//! String attributes coerce to the field type (`SmolStr`, `Duration` from
//! `"30s"`, `Option<T>`, enum unit variants), so a reflect component is usually
//! authorable without a template. A `<Tag>`'s children land as its direct
//! children (slots are transparent), so a child-reading handler like
//! `{ExchangeSequence}` reads them:
//! `<Route path="deploy" {ExchangeSequence}><MyBlock/><MyAction/></Route>`.
//!
//! An entry document has a single root element. Beside it may sit only the
//! declarations its launch pre-scans (`<RepoRoot>`, `<TemplateDir>`,
//! `<RequireCfg>`, `<Secrets>`, and whatever prescan type `app.install_type::<T>()`
//! adds), with string attributes and no `bx:cfg`: each acts before the build
//! and is consumed rather than built, so a root admitting no such child still
//! states what it needs. Anything else there is a second root, an error
//! naming every root found.
//!
//! ```text
//! <RequireCfg cfg="feature:extra"/>
//! <Sequence {CallOnReady}><Greet{name:"world"}/></Sequence>
//! ```
//!
//! A sequence route that skipped every child fails naming its unregistered
//! tags, and `beet check` elevates [`Unregistered`] to an error;
//! `Unregistered::allow` opts out a tag whose whole content is the missing
//! behavior (`<LiveReloadScript/>`). `bx:cfg` atoms are `namespace:argument`
//! resolved through the `BsxConditions` seam (`feature:`, `version:`, `env:` in
//! core, downstream registers its own) with Rust's `#[cfg]` operators and
//! precedence, and an unknown namespace is a hard error, never a silent false.

mod bsx_plugin;
mod directive;
mod entry;
mod parse;
mod reflect;
mod registry;
mod resolve;

pub use bsx_plugin::*;
pub use directive::*;
pub use parse::*;
pub use reflect::*;
pub use registry::*;
pub use resolve::*;
