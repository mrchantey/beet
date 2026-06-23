//! The self-contained stack-of-cards plugin.

use crate::prelude::*;
use beet_core::prelude::*;
#[cfg(feature = "std")]
use beet_ui::prelude::*;

/// A self-contained plugin turning a router into a HyperCard-style stack of
/// cards: a [`CardDeck`] marker opts a router in, and everything the stack needs
/// rides along here.
///
/// What it wires (all gated on a [`CardDeck`] being present, so a plain router is
/// untouched):
/// - keyboard [`card_nav`]: arrow/space/page keys step the in-world [`Navigator`]
///   between sibling cards (the docs-TUI keeps its plain-arrow page scroll).
/// - the [`card_notes`] pre-render hook: a card's back-of-card notes (after its
///   first `<hr>`) never render.
/// - the [`card_rules`] layout: the per-card frame and its body layouts, added to
///   the shared [`RuleSet`] so they compose with the material set.
///
/// A deck's in-world navigator defaults to its home route like any other (no
/// boot-time auto-open of the first card); a deck declares its opening content with
/// an `index` card.
///
/// Add it after `MaterialStylePlugin` so a card rule wins a cascade tie with the
/// material set; later plugins may extend the same rule set again to refine it.
#[derive(Default)]
pub struct CardStackPlugin;

impl Plugin for CardStackPlugin {
	fn build(&self, app: &mut App) {
		// `CardDeck` is markup-declarable (`<Router {(CardDeck, ..)}>`), so register
		// it for reflect; reflection works on bare metal, so register unconditionally.
		app.register_type::<CardDeck>();

		// the in-world navigator, the live page pipeline and the rule set are all
		// std-only (they need beet_ui), so the card-stack runtime is too. no_std
		// routers only ever see the reflect registration above.
		#[cfg(feature = "std")]
		{
			// the card-stack layout rules: extend the shared rule set the idiomatic
			// way (as the material/style plugins do), so they compose with — and on a
			// tie override — the material set, and stay extensible by later plugins.
			app.world_mut()
				.get_resource_or_init::<RuleSet>()
				.extend_rules(card_rules());

			// back-of-card notes pre-render hook: strip a card's first top-level
			// `<hr>` and the notes after it. Runs before the cascade so the notes are
			// gone before styling/highlighting/layout, and only while a `CardDeck`
			// router is present so non-deck content keeps its `<hr>` (the per-request
			// render tree is detached, so the gate is world-level, not an ancestor walk).
			app.add_systems(
				PostParseTree,
				card_notes
					.before(ResolveStylesSet)
					.run_if(any_with_component::<CardDeck>),
			);
		}

		// keyboard card nav rides the terminal input layer. The message registration
		// is idempotent and lets the system validate even with no input plugin
		// composed in. `card_nav` steps between sibling cards, gated on a `CardDeck`
		// router so the docs TUI keeps its plain-arrow page scroll.
		#[cfg(feature = "tui")]
		app.add_message::<bevy::input::keyboard::KeyboardInput>()
			.add_systems(Update, card_nav);
	}
}

