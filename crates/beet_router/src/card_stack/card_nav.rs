//! Keyboard navigation through a stack of cards.
//!
//! A [`CardDeck`] is a stack of cards (the HyperCard model): a flat list of
//! sibling routes the user steps through in order. [`CardNav`] is a single step
//! through that stack, resolved against the routes at the current card's level.
//! The [`card_nav`] system maps the arrow / space / page / home / end keys to
//! these steps for a [`CardDeck`] router driven by an in-world [`Navigator`].

use crate::prelude::*;
use beet_core::prelude::*;
#[cfg(feature = "tui")]
use beet_net::prelude::*;

/// Marker opting a router into a stack of cards: a flat list of sibling routes
/// the user steps through like a HyperCard stack. Read by [`card_nav`] (the
/// arrow/space/page keys step between cards) and the `card_notes` strip hook
/// (a card's back-of-card notes never render), both wired by [`CardStackPlugin`].
///
/// Opt-in via markup, eg `<Router {(CardDeck, ..)}>`: a deck declares it so the
/// stack machinery activates, while ordinary `serve`/docs-TUI routers omit it, so
/// their plain arrows keep scrolling the page.
#[derive(Debug, Default, Clone, Copy, Component, Reflect)]
#[reflect(Default, Component)]
pub struct CardDeck;

/// A single step through a [`CardDeck`], resolved against the flat list of
/// sibling cards at the current card's level. Unlike [`NavigateTo`]'s sibling
/// moves these clamp at the ends rather than wrapping: a stack of cards does not
/// loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum CardNav {
	/// The previous card, clamped at the first.
	Prev,
	/// The next card, clamped at the last.
	Next,
	/// The first card in the stack.
	First,
	/// The last card in the stack.
	Last,
}

/// The ordered cards of a deck: the router's own page route — the `/` home card
/// a deck declares via an `index` content file — first, then its page-route
/// children, in tree order.
///
/// Only user-facing [`PageRoute`](crate::prelude::PageRoute)s are cards: the
/// infrastructure routes a deck serves alongside its slides (`/health`, the
/// reactivity-runtime asset, the `client_io` websocket, a mounted blob store of
/// assets) are not steppable cards, so they never become the opening card or land
/// in the stack.
fn deck_cards(tree: &RouteTree) -> Vec<&RouteTree> {
	let mut cards = Vec::new();
	// the deck's home card is the `/` route (an `index` content file): usually the
	// router's own node, but a store-discovered `index` can instead land as an
	// empty-path child. Either way it is listed once, first, so the deck opens and
	// steps from it like any other card.
	let home = if tree.node().is_some_and(|node| node.is_page_route) {
		Some(tree)
	} else {
		tree.children.iter().find(|child| {
			is_empty_path(child)
				&& child.node().is_some_and(|node| node.is_page_route)
		})
	};
	cards.extend(home);
	// the remaining page-route children in tree order, excluding any empty-path
	// home child (handled above). Without this, a deck whose `index` sorts last
	// among the slides appends a duplicate empty-path card after the final slide,
	// so stepping `Next` off the last card lands on the blank `/` clone and back —
	// the last-card → empty-page oscillation.
	cards.extend(tree.children.iter().filter(|child| {
		!is_empty_path(child)
			&& child.node().is_some_and(|node| node.is_page_route)
	}));
	cards
}

/// Whether `card` sits at the deck root (an empty path, ie the `/` home card).
fn is_empty_path(card: &RouteTree) -> bool { card.path.iter().count() == 0 }

/// Whether `card`'s full path equals `path` by segment name, so the empty-path
/// home card matches `/` and a child card matches its own path.
fn card_path_matches(card: &RouteTree, path: &[SmolStr]) -> bool {
	card.path.iter().count() == path.len()
		&& card
			.path
			.iter()
			.zip(path)
			.all(|(seg, want)| seg.name() == want.as_str())
}

/// The path of the `n`th (1-based) card, clamped to the stack range (`n < 1`
/// lands on the first, `n > len` on the last). Backs the initial-card `--slide=N`
/// patch in [`CardStackPlugin`].
pub fn resolve_nth_card(tree: &RouteTree, n: usize) -> Result<Vec<SmolStr>> {
	let cards = deck_cards(tree);
	if cards.is_empty() {
		bevybail!("card stack has no cards");
	}
	let idx = n.saturating_sub(1).min(cards.len() - 1);
	path_segments(&cards[idx].path)
}

/// Resolve a [`CardNav`] step against a [`RouteTree`] from the current path,
/// returning the target path segments. Clamps at the ends (no wrap).
pub fn resolve_card(
	tree: &RouteTree,
	current_path: &[SmolStr],
	nav: CardNav,
) -> Result<Vec<SmolStr>> {
	let cards = deck_cards(tree);
	if cards.is_empty() {
		bevybail!("card stack has no cards");
	}
	// the current card is the one whose full path matches the navigator's path;
	// the home card matches the empty path (`/`).
	let current_idx = cards
		.iter()
		.position(|card| card_path_matches(card, current_path))
		.ok_or_else(|| {
			bevyhow!(
				"current path /{} is not a card in this deck",
				current_path.join("/")
			)
		})?;
	let target_idx = match nav {
		CardNav::Prev => current_idx.saturating_sub(1),
		CardNav::Next => (current_idx + 1).min(cards.len() - 1),
		CardNav::First => 0,
		CardNav::Last => cards.len() - 1,
	};
	path_segments(&cards[target_idx].path)
}

/// System: arrow / space / page / home / end keys step the navigator between
/// sibling cards, opt-in via a [`CardDeck`] marker on the in-world router.
///
/// Key map (no wrap, clamped at the ends):
/// - prev: Left, Up, PageUp
/// - next: Right, Down, PageDown, Space, Enter
/// - first: Home  |  last: End
///
/// Gated so it never hijacks the docs-TUI: only a navigator whose
/// [`NavigatorTransport::InWorld`] router carries [`CardDeck`] navigates, and
/// Alt+arrows are left to [`nav_shortcuts`](crate::prelude::nav_shortcuts)
/// (history), so a plain-arrow docs TUI keeps scrolling.
#[cfg(feature = "tui")]
pub(crate) fn card_nav(
	mut keys: MessageReader<bevy::input::keyboard::KeyboardInput>,
	navigators: Query<(Entity, &Navigator)>,
	decks: Query<(), With<CardDeck>>,
	route_trees: Query<&RouteTree>,
	mut commands: Commands,
) {
	use bevy::input::ButtonState;
	use bevy::input::keyboard::KeyCode;

	// classify this frame's keys into at most one card step, skipping the
	// Alt-modified arrows that drive history in `nav_shortcuts`.
	let (mut alt, mut nav) = (false, None);
	for key in keys.read().filter(|key| key.state == ButtonState::Pressed) {
		match key.key_code {
			KeyCode::AltLeft | KeyCode::AltRight => alt = true,
			KeyCode::ArrowLeft | KeyCode::ArrowUp | KeyCode::PageUp => {
				nav = Some(CardNav::Prev)
			}
			KeyCode::ArrowRight
			| KeyCode::ArrowDown
			| KeyCode::PageDown
			| KeyCode::Space
			| KeyCode::Enter => nav = Some(CardNav::Next),
			KeyCode::Home => nav = Some(CardNav::First),
			KeyCode::End => nav = Some(CardNav::Last),
			_ => {}
		}
	}

	// every guard below is a clean no-op (never an error, so no log spam): a
	// non-card key or Alt-arrow, no/HTTP navigator, an unmarked or not-yet-built
	// router (eg the docs TUI), or a resolution miss at a non-card path.
	let Some((nav, (entity, navigator))) =
		nav.filter(|_| !alt).zip(navigators.single().ok())
	else {
		return;
	};
	let NavigatorTransport::InWorld { router } = navigator.transport() else {
		return;
	};
	let (true, Ok(tree)) = (decks.contains(*router), route_trees.get(*router))
	else {
		return;
	};
	let current_path = navigator.current_url().path().clone();
	let Ok(target) = resolve_card(tree, &current_path, nav) else {
		return;
	};

	// navigate to the resolved card's absolute path (clamped at the stack's ends).
	let url = Url::parse(format!("/{}", target.join("/")));
	commands.entity(entity).queue_async(async move |entity| {
		Navigator::navigate_to(entity, url).await
	});
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	/// A minimal router bundle: the [`Router`] dispatch action and nothing else,
	/// so the sibling ordering these tests assert is not perturbed by opinionated
	/// app routes.
	fn nav_router() -> impl Bundle { (Router, NavigateHandler::default()) }

	/// A flat three-card stack, the [`RouteTree`] the card resolvers resolve
	/// against.
	fn card_stack() -> RouteTree {
		let mut world = router_world();
		let root = world
			.spawn((nav_router(), children![
				// cards are page routes (the marker `RoutesDir` content gets), so the
				// page-only card resolvers see them; a bare route would be filtered out.
				(
					render_action::fixed_func_route(
						"alpha",
						|| rsx! { <p>"a"</p> }
					),
					PageRoute
				),
				(
					render_action::fixed_func_route(
						"beta",
						|| rsx! { <p>"b"</p> }
					),
					PageRoute
				),
				(
					render_action::fixed_func_route(
						"gamma",
						|| rsx! { <p>"c"</p> }
					),
					PageRoute
				),
			]))
			.flush();
		world.entity(root).get::<RouteTree>().unwrap().clone()
	}

	/// The single path segment a [`CardNav`] step lands on from `current`.
	fn card_to(tree: &RouteTree, current: &str, nav: CardNav) -> String {
		resolve_card(tree, &[current.into()], nav)
			.unwrap()
			.join("/")
	}

	/// Next/prev clamp at the ends rather than wrapping (a stack does not loop).
	#[beet_core::test]
	fn card_next_prev_clamp_at_ends() {
		let tree = card_stack();
		// interior moves
		card_to(&tree, "alpha", CardNav::Next).xpect_eq("beta");
		card_to(&tree, "beta", CardNav::Next).xpect_eq("gamma");
		card_to(&tree, "gamma", CardNav::Prev).xpect_eq("beta");
		// next on the last card stays put; prev on the first stays put
		card_to(&tree, "gamma", CardNav::Next).xpect_eq("gamma");
		card_to(&tree, "alpha", CardNav::Prev).xpect_eq("alpha");
	}

	/// First/last jump to the ends regardless of the current card.
	#[beet_core::test]
	fn card_first_and_last() {
		let tree = card_stack();
		card_to(&tree, "beta", CardNav::First).xpect_eq("alpha");
		card_to(&tree, "beta", CardNav::Last).xpect_eq("gamma");
		card_to(&tree, "alpha", CardNav::First).xpect_eq("alpha");
		card_to(&tree, "gamma", CardNav::Last).xpect_eq("gamma");
	}

	/// `--slide=N` maps a 1-based index to the Nth ordered card, clamping an
	/// out-of-range N to the first/last rather than erroring.
	#[beet_core::test]
	fn nth_card_maps_and_clamps() {
		let tree = card_stack();
		let nth = |n| resolve_nth_card(&tree, n).unwrap().join("/");
		// 1-based: card 1 is the first, 3 the last
		nth(1).xpect_eq("alpha");
		nth(2).xpect_eq("beta");
		nth(3).xpect_eq("gamma");
		// out of range clamps to the ends
		nth(0).xpect_eq("alpha");
		nth(99).xpect_eq("gamma");
	}

	/// A deck whose `/` is a slide (an empty-path `index` card on the router's own
	/// node) plus two child cards. The empty-path route lands on the tree root, so
	/// it is the deck's home card, not a child.
	fn card_stack_with_home() -> RouteTree {
		let mut world = router_world();
		let root = world
			.spawn((nav_router(), children![
				// empty path → the route attaches to the tree root: the home card.
				(
					render_action::fixed_func_route("", || rsx! {
						<p>"home"</p>
					}),
					PageRoute
				),
				(
					render_action::fixed_func_route("alpha", || rsx! {
						<p>"a"</p>
					}),
					PageRoute
				),
				(
					render_action::fixed_func_route("beta", || rsx! {
						<p>"b"</p>
					}),
					PageRoute
				),
			]))
			.flush();
		world.entity(root).get::<RouteTree>().unwrap().clone()
	}

	/// The `/` home card is the first card: `--slide=1` opens it, and prev clamps
	/// on it rather than stepping off the deck.
	#[beet_core::test]
	fn home_card_is_first() {
		let tree = card_stack_with_home();
		// the home card (empty path) is card 1, the children follow.
		resolve_nth_card(&tree, 1).unwrap().join("/").xpect_eq("");
		resolve_nth_card(&tree, 2).unwrap().join("/").xpect_eq("alpha");
		resolve_nth_card(&tree, 3).unwrap().join("/").xpect_eq("beta");
	}

	/// Stepping in and out of the `/` home card: next leaves it for the first
	/// child, prev returns to it, and prev on it clamps (the deck does not wrap).
	#[beet_core::test]
	fn home_card_steps_and_clamps() {
		let tree = card_stack_with_home();
		// from the home card (empty current path), next → first child, prev clamps.
		resolve_card(&tree, &[], CardNav::Next)
			.unwrap()
			.join("/")
			.xpect_eq("alpha");
		resolve_card(&tree, &[], CardNav::Prev)
			.unwrap()
			.join("/")
			.xpect_eq("");
		// from the first child, prev returns to the home card.
		resolve_card(&tree, &["alpha".into()], CardNav::Prev)
			.unwrap()
			.join("/")
			.xpect_eq("");
		// first/last still reach the deck ends.
		resolve_card(&tree, &["alpha".into()], CardNav::First)
			.unwrap()
			.join("/")
			.xpect_eq("");
		resolve_card(&tree, &[], CardNav::Last)
			.unwrap()
			.join("/")
			.xpect_eq("beta");
	}
}
