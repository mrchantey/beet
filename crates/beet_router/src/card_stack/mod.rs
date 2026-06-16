//! A reusable, self-contained stack of cards (the HyperCard model).
//!
//! A [`CardDeck`] turns a router into a *stack of cards*: a flat list of sibling
//! routes the user steps through in order, like the cards of a HyperCard stack.
//! It is not presentation-specific — the same machinery suits a settings menu or
//! a future up/down between decks — though a slide deck is the obvious first use.
//!
//! Everything the stack needs lives here and rides in on one plugin:
//! - [`CardStackPlugin`] — wires it all, gated on a [`CardDeck`] being present.
//! - [`CardDeck`] / [`CardNav`] — the opt-in marker and a single navigation step.
//! - [`resolve_card`] / [`resolve_nth_card`] — resolve a step / the Nth card.
//! - [`card_notes`] — strip a card's hidden back-of-card content (after its first
//!   `<hr>`) before render.
//! - [`card_rules`] — the per-card layout, contributed to the shared rule set.
//!
//! Other code depends on `card_stack`, never the reverse: the `navigate`/`router`
//! core stays free of any card specifics.

mod card_nav;
pub use card_nav::*;
// std-only: the card-stack runtime (the plugin, the back-of-card notes hook, the
// layout rules) needs beet_ui; no_std routers only see the no_std items above.
#[cfg(feature = "std")]
mod card_notes;
#[cfg(feature = "std")]
pub use card_notes::*;
#[cfg(feature = "std")]
mod card_stack_plugin;
#[cfg(feature = "std")]
pub use card_stack_plugin::*;
#[cfg(feature = "std")]
mod card_styles;
#[cfg(feature = "std")]
pub use card_styles::*;
