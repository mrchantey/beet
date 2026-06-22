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

/// The path of the `n`th (1-based) top-level card, clamped to the stack range
/// (`n < 1` lands on the first, `n > len` on the last). A stack is a flat list,
/// so its cards are the routable children at the tree root, in sorted order.
/// Backs the initial-card `--slide=N` patch in [`CardStackPlugin`].
pub fn resolve_nth_card(tree: &RouteTree, n: usize) -> Result<Vec<SmolStr>> {
	// only user-facing page routes are cards: the infrastructure routes a deck
	// serves alongside its slides (`/health`, the reactivity-runtime asset, the
	// `client_io` websocket, a mounted blob store of assets) are not steppable
	// cards, so they never become the opening card or land in the stack.
	let cards: Vec<&RouteTree> = tree
		.children
		.iter()
		.filter(|child| child.node().is_some_and(|node| node.is_page_route))
		.collect();
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
	let (siblings, current_idx) =
		siblings_and_index(tree, current_path, true)?;
	let target_idx = match nav {
		CardNav::Prev => current_idx.saturating_sub(1),
		CardNav::Next => (current_idx + 1).min(siblings.len() - 1),
		CardNav::First => 0,
		CardNav::Last => siblings.len() - 1,
	};
	path_segments(&siblings[target_idx].path)
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
}
