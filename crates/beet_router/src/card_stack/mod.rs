//! A reusable, self-contained stack of cards (the HyperCard model).
//!
//! A [`CardDeck`] turns a router into a *stack of cards* the user steps
//! through in order, like the cards of a HyperCard stack. It is not
//! presentation-specific, the same machinery suits a settings menu, though a
//! slide deck is the obvious first use.
//!
//! A route and a card article nest rather than compete. A route whose
//! document holds no `article data-card` is one card, so a deck of markdown
//! files keeps a route per card, and a route holding some is a card per
//! article, so a `.pptx` is one route of a card per slide. A markdown file
//! authors several cards the same way, each an `<article data-card>` block,
//! its speaker notes after its `---`:
//!
//! ```md
//! <article data-card>
//!
//! # Gentle slopes
//!
//! Most tools ask little of a beginner.
//!
//! ---
//!
//! Start with the spreadsheet everyone has used.
//!
//! </article>
//!
//! <article data-card>
//!
//! # Jagged cliffs
//!
//! - then a wall of syntax
//!
//! ---
//!
//! Ask who has hit one.
//!
//! </article>
//! ```
//!
//! The position is a route and a card within it ([`CardPosition`]), the card
//! addressed by the `card` request param and the stack's by `slide`, which
//! counts cards across routes. Both are indices from zero, as every index
//! is: `talk?card=1` is the route's second card, `--slide=4` the stack's
//! fifth. Both are declared as [`CardParams`].
//!
//! Everything the stack needs rides in on one plugin:
//! - [`CardStackPlugin`]: registers the marker and its params, the layout rules
//!   and the keyboard steps.
//! - [`CardDeck`]: the opt-in marker, whose render middleware shows one card
//!   and hides its notes, and whose request middleware answers a request for
//!   the home card accepting a slide deck with the whole stack.
//! - [`CardNav`]: a single step through the stack, [`CardNav::resolve_nth`] the
//!   Nth card.
//! - [`card_rules`]: the per-card layout, contributed to the shared rule set.
//!
//! Other code depends on `card_stack`, never the reverse: the `navigate`/`router`
//! core stays free of any card specifics.

mod card_nav;
pub use card_nav::*;
// std-only: the card-stack runtime (the plugin, the render middleware, the
// layout rules) needs beet_ui; no_std routers only see the no_std items above.
#[cfg(feature = "std")]
mod card_render;
#[cfg(feature = "std")]
pub use card_render::CardCount;
#[cfg(feature = "std")]
mod card_stack_plugin;
#[cfg(feature = "std")]
pub use card_stack_plugin::*;
#[cfg(feature = "std")]
mod card_styles;
#[cfg(feature = "std")]
pub(crate) use card_styles::*;
