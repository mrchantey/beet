//! The self-contained stack-of-cards plugin.

use crate::prelude::*;
use beet_core::prelude::*;
#[cfg(feature = "std")]
use beet_net::prelude::*;
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
/// - the initial-card patch: when a deck's in-world navigator boots, it is sent
///   to its opening card (`--slide=N`, else the first card).
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

			// send a deck's in-world navigator to its opening card once it boots.
			app.add_observer(open_initial_card);
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

/// Observer: when an in-world [`Navigator`] for a [`CardDeck`] router boots, send
/// it to the deck's opening card.
///
/// [`TuiServer`](crate::prelude::TuiServer) boots a card-agnostic navigator at a
/// generic home (`/`); this patches it up from downstream. The opening card is
/// `--slide=N` (1-based, clamped via [`resolve_nth_card`]) when given, else the
/// first card. The navigator's own home navigation runs first (a component hook,
/// before this observer), so a brief generic first-paint before the card is
/// possible; the card navigation lands last and wins.
///
/// A deck's cards are discovered asynchronously (`RoutesDir` lists its directory
/// off the runtime, so the card routes appear a few ticks after boot), and this
/// observer fires the moment the navigator is added — typically before any card
/// exists. So the open is queued as an async task that polls the router's route
/// tree for a short window until a card resolves, rather than resolving once up
/// front and giving up; without this the navigator is stranded on the generic
/// home (which a deck has no route for). A failed open logs rather than crashing
/// the host, mirroring the navigator's graceful home-load handling.
#[cfg(feature = "std")]
fn open_initial_card(
	ev: On<Add, Navigator>,
	navigators: Query<&Navigator>,
	decks: Query<(), With<CardDeck>>,
	mut commands: Commands,
) {
	// only an in-world navigator pointed at a `CardDeck` router opens a card; an
	// HTTP navigator or a plain router is left at its generic home.
	let Ok(navigator) = navigators.get(ev.entity) else {
		return;
	};
	let NavigatorTransport::InWorld { router } = navigator.transport() else {
		return;
	};
	let router = *router;
	if !decks.contains(router) {
		return;
	}

	// `--slide=N` (1-based) over the first card. Read from the launch argv, the
	// downstream stand-in for the deck-specific boot config the server no longer
	// owns.
	let slide = Request::from_cli_args(CliArgs::parse_env())
		.get_param("slide")
		.and_then(|val| val.parse().ok())
		.unwrap_or(1);

	commands.entity(ev.entity).queue_async(async move |entity| {
		let world = entity.world().clone();
		// poll for the async-discovered cards for up to ~1s, then open the Nth.
		for _ in 0..50 {
			let path = world
				.with(move |world: &mut World| {
					world
						.get_entity(router)
						.ok()
						.and_then(|router| router.get::<RouteTree>())
						.and_then(|tree| resolve_nth_card(tree, slide).ok())
						.map(|segments| format!("/{}", segments.join("/")))
				})
				.await;
			match path {
				Some(path) => {
					if let Err(err) =
						Navigator::navigate_to(entity, Url::parse(path.clone()))
							.await
					{
						error!("failed to open initial card `{path}`: {err}");
					}
					return Ok(());
				}
				// cards not discovered yet: wait a tick and retry.
				None => time_ext::sleep_millis(20).await,
			}
		}
		error!("deck navigator found no cards to open");
		Ok(())
	});
}
