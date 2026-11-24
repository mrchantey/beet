//! Syntax tree to scene body: the one lowering every markup format shares.
//!
//! [`BsxParser::lower`] turns a [`BsxNode`] tree into a [`SceneBody`],
//! touching no world. What it consults beside the nodes travels with the
//! format registry ([`DocParsers`]): the directive set, rewritten away before
//! anything else, and the type registry the names resolve against, which is
//! data and no world.
//!
//! | markup | body |
//! | --- | --- |
//! | a lowercase tag | an entity with its [`Element`] and an [`Attribute`] entity per attribute, in file order |
//! | `<slot/>`, `slot="name"` | [`SlotTarget`], [`SlotChild`] |
//! | a component tag or spread | the component, patched from its attributes or literal |
//! | a `#[template]` tag or spread | its invocation, expanded when the body spawns; an attribute beyond its props an attribute entity, which the root element it builds takes |
//! | a resource tag or spread | its fields on the entity, which patch the live resource as the body spawns |
//! | a name nothing registered | an entry keyed by the name as written, which the spawn holds in [`Unregistered`] |
//! | an `@` binding | the sync components of its source |
//! | `$name` | the key of the entity `bx:ref="name"` names |
//! | `bx:cfg` | [`Cfg`], answered when the body spawns |
//!
//! A value is written as its type's own serde form, only the fields the
//! markup authored, the rest filling from the type's default when the body
//! spawns. A value no serde form can hold stays as authored, which a spawn
//! reads through the same coercions ([`SceneBody::reflect`]).
//!
//! # Keys
//!
//! A text format stores no keys, so each is minted from structure: the
//! parent's key, `/`, the tag (`#text` for a text node, `@name` for an
//! attribute) and the entity's index among siblings of the same tag,
//! `0/main.0/p.3`. Inserting a `<div>` so leaves every `<p>`'s key alone.
//! `bx:ref="name"` replaces the whole key with the name, which then survives
//! a move, its descendants keying under it.
//!
//! # The root
//!
//! A document with one root element is that element: the body's first entity
//! is it, so its spreads are the root's components. Comments beside the root
//! document the file and build nothing. Anything else, several roots or text,
//! is a bare root holding each as a child.
mod binding;
mod element;
mod literal;
mod lowering;
mod markup;
mod patch;

pub use element::*;
pub(super) use literal::literal_to_value;
pub(super) use lowering::*;
