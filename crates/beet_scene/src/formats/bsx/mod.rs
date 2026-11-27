//! BSX: one hand-written recursive-descent cursor parser whose
//! disabled-surface configuration is exactly HTML, and the lowering of its
//! syntax tree to a [`SceneBody`].
//!
//! BSX is the full grammar (uppercase resolution, the value grammar, `bx:`
//! directives); HTML is BSX with that surface disabled. The parser is
//! XML-inspired markup, not "an HTML subset": one grammar, with HTML the markup
//! it accepts when the extra surface is switched off. A document becomes a
//! [`BsxNode`] syntax tree ([`BsxNode::parse_document`], where a tool that edits
//! markup stops), its directives are rewritten, and the result lowers to the
//! scene body every other format reads to. No world is touched until that body
//! spawns: [`BsxParser`] is a [`DocParser`] like any other.
//!
//! Internal split, one directory per stage: source text to syntax tree is
//! `beet_core`'s (the cursor, the markup parser, the value grammar), so a
//! crate below this one reads markup without lowering it, syntax tree to
//! explicit syntax tree in `directive` (every `bx:` directive rewritten away),
//! syntax tree to scene body in `lower` (with prop-schema verification), and
//! authored literal to the value it holds in the crate's `literal` module,
//! which every authored format shares.
//!
//! # Syntax
//!
//! Lowercase tags are HTML [`Element`]s; attributes become attribute child
//! entities; text becomes a [`Value`]. A braced `{..}` block is the value
//! grammar: a `{literal}` lowers to a [`Value`], an `{@source:path}` binding to
//! its reactive sync components. `<script>` and `<style>` read their body as
//! raw text, HTML's own rule. An uppercase tag or a spread nothing is
//! registered under is not an error:
//! it is held with its value in the entity's [`Unregistered`] and, for
//! a tag, builds its children anyway, so a lean binary loads the same document
//! shape a full one does and only its behavior is missing.
//!
//! [`bx:cfg`](BuildCondition) is the one directive that breaks that last
//! sentence, deliberately: it lowers to a [`Cfg`], and a subtree whose condition
//! is unmet is pruned from the body as it spawns, so an excluded branch has no
//! shape to load. Structure stays universal by default because an inert entity
//! costs nothing; a node that performs an EFFECT as it builds (an [`Instance`]
//! reading a file) has no inert form, and `bx:cfg` is how such a branch is kept
//! out of a build that must not run it. It leaves a [`CfgExcluded`] tombstone,
//! so an excluded route still reports why it is missing rather than 404ing.
//! Asserting the same condition instead of applying it is
//! [`RequireCfg`], which refuses the whole load.
//!
//! ## Entities and keys
//!
//! A document whose one root is an element is that element: its components are
//! the spawned root's, and its spreads the document's metadata. Any other
//! document (several roots, text beside an element) is a bare root holding them.
//! Each entity of the body is keyed by where it sits (`0/section.0/p.1`, minted as
//! the `lower` module docs state), so the same markup always keys the same.
//! `bx:ref="name"` replaces the key with `name`, which `$name` then references
//! from anywhere in the document, before or after.
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
//! - `@prop:` the props of the document it is written in, what the [`Instance`]
//!   using that document passed (a bound prop chains reactively), in content
//!   slotted into another document as anywhere else
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
//! between parsing and the lowering ([`BsxDirectives`]), so the scene, a save and
//! the runtime never learn it existed. The line between the two kinds of
//! shorthand is the world: a directive needs none and runs as a document
//! lowers, where a `#[template]` needs one and runs at spawn. A rewrite's
//! output is rewritten in turn, so sugar may stand on sugar. The standard
//! set:
//!
//! ```text
//! bx:ref="name"                      the entity's key
//! bx:cfg="feature:infra"             {Cfg{cfg:"feature:infra"}}
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
//! nothing matches refuses the document.
//!
//! ```
//! use beet_core::prelude::*;
//! use beet_scene::prelude::*;
//!
//! let mut world = (ScenePlugin, DocumentPlugin).into_world();
//! let section = BsxParser::spawn(
//!     &mut world,
//!     r#"<section bx:scope="user"><p>{@doc:name="Ada"}</p><button bx:onclick="target.with_field('clicks', clicks => clicks + 1)">+</button></section>"#,
//! )
//! .unwrap();
//! world.update_local();
//!
//! // the `bx:scope` directive is a `DocumentScope`, so the text binding
//! // resolves `user.name`, initialized from the binding's `=init`
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
//! ## Templates and documents
//!
//! An uppercase tag resolves by name against the type registry: a component, a
//! `#[template]`, or a `#[reflect(Resource)]` type (`<PackageConfig title=".."/>`
//! patches the live resource's named fields, producing no markup). Another
//! *document* is never a tag: it is an [`Instance`], naming it by a [`DocPath`],
//! relative to the document it is written in when it starts with `./` or `../`
//! and to the package (the directory the entry is in) otherwise, the extension
//! optional where one file matches and written by convention.
//!
//! ```text
//! <!-- widgets/Card.bsx -->
//! <Prop name="title" kind="string" default="Untitled"/>
//! <section><h2>{@prop:title}</h2><slot/></section>
//!
//! <!-- a page -->
//! <Instance document="widgets/Card.bsx" props={{title:"Intro"}}><p>hi</p></Instance>
//! ```
//!
//! The document is read once and held as data, and each use builds it under the
//! use site with entities of its own: `props` fill its `@prop:` bindings, bound
//! or literal, a [`Prop`] declares one with its default, and the use's children
//! fill its `<slot>`s. A document instantiating itself, however indirectly, is
//! refused; recursion over data is a list's. Activation is the `instance`
//! module's.
//!
//! A template's root element is the tag's own entity, so an attribute beyond its
//! declared props falls through to that element: `<Link href="/docs" hidden>`
//! renders `<a href="/docs" hidden class="btn ..">`. A `class` joins the
//! element's own, any other key replaces its value, and a template with no
//! single root element refuses the attribute rather than dropping it.
//!
//! A component may hold its children unbuilt, as content it builds when it
//! chooses, by implementing [`HoldChildren`]: each child lowers into the field
//! its `slot` names, a scene body within the body.
//!
//! The remaining surface (bare-position component spreads `{(A, B)}`, the `$ref`
//! entity references, enum/struct/list literals) lives in the value grammar
//! (`beet_core`'s) and its reflection (`literal`).
//!
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
//! emits no element.
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
//! The declarations an entry's launch pre-scans (`<PackageRoot>`, `<RequireCfg>`,
//! `<Secrets>`, and whatever prescan type `app.install_types::<T>()` adds), with
//! string attributes and no `bx:cfg`, each act before the build. A root
//! admitting no such child has them beside it:
//!
//! ```text
//! <RequireCfg cfg="feature:extra"/>
//! <Sequence {CallOnReady}><Greet{name:"world"}/></Sequence>
//! ```
//!
//! A sequence route that skipped every child fails naming its unregistered
//! tags, and `beet check` elevates [`Unregistered`] to an error. `bx:cfg` atoms are `namespace:argument`
//! resolved through the `BsxConditions` seam (`feature:`, `version:`, `env:` in
//! core, downstream registers its own) with Rust's `#[cfg]` operators and
//! precedence, and an unknown namespace is a hard error, never a silent false.

#[cfg(doc)]
use crate::prelude::*;
#[cfg(doc)]
use beet_core::prelude::*;

mod bsx_parser;
mod directive;
mod lower;

pub use bsx_parser::*;
pub use directive::*;
pub use lower::*;
