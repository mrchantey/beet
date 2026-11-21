//! The self-contained stack-of-cards plugin.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

/// A self-contained plugin turning a router into a HyperCard-style stack of
/// cards: a [`CardDeck`] marker opts a router in, and everything the stack needs
/// rides along here. [`RouterPlugin`] installs it, every piece gated on a
/// [`CardDeck`], so a plain router is untouched.
///
/// What it wires:
/// - the [`CardDeck`] marker for markup, and its [`CardParams`] at subtree
///   scope, so every route of a deck takes `--card` and `--slide`. The
///   marker's own hook pushes its render and request middleware.
/// - the [`card_rules`] layout: the per-card frame and its body layouts, added to
///   the shared [`RuleSet`] so they compose with the material set.
/// - keyboard [`card_nav`]: arrow/space/page keys step the in-world
///   [`Navigator`] through the stack (the docs-TUI keeps its plain-arrow page
///   scroll).
#[derive(Default)]
pub struct CardStackPlugin;

impl Plugin for CardStackPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<CardDeck>()
			.install(SubtreeParams::declare_on::<CardDeck, CardParams>());
		// the card-stack layout rules extend the shared rule set, as the
		// material and style plugins do, so they compose with the material set
		app.world_mut()
			.get_resource_or_init::<RuleSet>()
			.extend_rules(card_rules());
		// keyboard card nav rides the terminal input layer. The message
		// registration is idempotent and lets the system validate with no input
		// plugin composed in.
		#[cfg(feature = "tui")]
		app.add_message::<bevy::input::keyboard::KeyboardInput>()
			.add_systems(Update, card_nav);
	}
}
