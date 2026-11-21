//! Stepping through a stack of cards.
//!
//! A [`CardDeck`] is a stack of cards (the HyperCard model): its page routes
//! in reading order, each one card, or a card per `article data-card` its
//! document holds. A [`CardPosition`] names one card, a route's path and the
//! card's place within it, and [`CardNav`] is a single step through the
//! stack. The [`card_nav`] system maps the arrow / space / page / home / end
//! keys to these steps for a [`CardDeck`] router driven by an in-world
//! [`Navigator`].

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Marker opting a router into a stack of cards: its page routes, and the card
/// articles within them, stepped through like a HyperCard stack. A render of a
/// deck's route shows one card and hides its notes, `--card` and `--slide`
/// address a card, a request for the home card accepting a slide deck answers
/// the whole stack, and in the terminal the arrow/space/page keys step between
/// cards, all wired by [`CardStackPlugin`].
///
/// Opt-in via markup, eg `<Router {(CardDeck, ..)}>`: a deck declares it so the
/// stack machinery activates, while ordinary `serve`/docs-TUI routers omit it, so
/// their plain arrows keep scrolling the page.
#[derive(Debug, Default, Clone, Copy, Component, Reflect)]
#[reflect(Default, Component)]
#[cfg_attr(feature = "std", component(on_add = CardDeck::on_add))]
pub struct CardDeck;

/// The params a [`CardDeck`] reads, declared on it at subtree scope so every
/// route of the deck takes them. Both count from zero, as every index does.
#[derive(Debug, Default, Clone, Reflect)]
pub struct CardParams {
	/// The index of the route's card to show, its first when unset.
	pub card: Option<usize>,
	/// The index of the stack's card to show, counting every route's cards
	/// in order.
	pub slide: Option<usize>,
}

/// One card of a deck: its route's path and its index among the route's
/// cards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardPosition {
	/// The route's path segments, empty for the home card.
	pub path: Vec<SmolStr>,
	/// The card's index within the route.
	pub card: usize,
}

/// A single step through a [`CardDeck`], resolved against its cards in order.
/// Unlike [`NavigateTo`]'s sibling moves these clamp at the ends rather than
/// wrapping: a stack of cards does not loop.
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

impl CardPosition {
	/// The first card of the route at `path`.
	pub fn first(path: &PathPattern) -> Self {
		Self {
			path: path
				.iter()
				.map(|segment| SmolStr::from(segment.name()))
				.collect(),
			card: 0,
		}
	}

	/// The url showing this card: its route's path, with `card` past the
	/// route's first.
	pub fn url(&self) -> Url {
		let url = Url::coerce(format!("/{}", self.path.join("/")));
		match self.card > 0 {
			true => url.with_param("card", self.card.to_string()),
			false => url,
		}
	}

	/// The card `url` shows in a deck of `positions`: the stack's card at
	/// `slide` when it names one, else its path's at `card`, its first when
	/// unset.
	pub fn of_url(positions: &[CardPosition], url: &Url) -> Self {
		let param = |key: &str| {
			url.params()
				.get(key)
				.and_then(|value| value.parse::<usize>().ok())
		};
		if let Some(slide) = param("slide")
			&& let Ok(position) = CardNav::resolve_nth(positions, slide)
		{
			return position;
		}
		Self {
			path: url.path().clone(),
			card: param("card").unwrap_or_default(),
		}
	}
}

impl CardNav {
	/// The card of `positions` at `index`, clamped to the last, so
	/// `--slide=N` counts cards across routes.
	pub fn resolve_nth(
		positions: &[CardPosition],
		index: usize,
	) -> Result<CardPosition> {
		positions
			.get(index)
			.or_else(|| positions.last())
			.cloned()
			.ok_or_else(|| bevyhow!("card stack has no cards"))
	}

	/// Resolve this step from `current` against `positions`, clamping at the
	/// ends (no wrap).
	pub fn resolve(
		self,
		positions: &[CardPosition],
		current: &CardPosition,
	) -> Result<CardPosition> {
		if positions.is_empty() {
			bevybail!("card stack has no cards");
		}
		// a card past its route's last is the route's last
		let index = positions
			.iter()
			.position(|position| position == current)
			.or_else(|| {
				positions
					.iter()
					.rposition(|position| position.path == current.path)
			})
			.ok_or_else(|| {
				bevyhow!(
					"current path /{} is not a card in this deck",
					current.path.join("/")
				)
			})?;
		let target = match self {
			Self::Prev => index.saturating_sub(1),
			Self::Next => (index + 1).min(positions.len() - 1),
			Self::First => 0,
			Self::Last => positions.len() - 1,
		};
		positions[target].clone().xok()
	}
}

impl CardDeck {
	/// The page routes of a deck in stack order: the `/` home card (an
	/// `index` content file) first, then every page route in reading order,
	/// so the stack steps across nested folders.
	///
	/// Only user-facing [`PageRoute`](crate::prelude::PageRoute)s are cards:
	/// the infrastructure routes a deck serves alongside them (`/health`, the
	/// `app-info` scene, the `client_io` websocket, a mounted blob store of
	/// assets) never land in the stack.
	pub fn routes(tree: &RouteTree) -> Vec<&ActionNode> {
		let nodes = tree.iter_dfs();
		let is_home = |tree: &&RouteTree| tree.path.iter().count() == 0;
		// the home card leads once, wherever its empty path sorts
		nodes
			.iter()
			.filter(|tree| is_home(tree))
			.chain(nodes.iter().filter(|tree| !is_home(tree)))
			.filter_map(|tree| tree.node().filter(|node| node.is_page_route))
			.collect()
	}

	/// Every card of `routes` in order, each route's card count by `count`,
	/// a route holding no card articles one card and one whose cards are all
	/// hidden none.
	pub fn positions_of(
		routes: &[&ActionNode],
		count: impl Fn(&ActionNode) -> usize,
	) -> Vec<CardPosition> {
		routes
			.iter()
			.flat_map(|node| {
				let first = CardPosition::first(&node.path);
				(0..count(node)).map(move |card| CardPosition {
					card,
					..first.clone()
				})
			})
			.collect()
	}
}

#[cfg(feature = "std")]
impl CardDeck {
	/// Every card of the deck `deck` declares, each route's cards counted by
	/// the [`CardCount`] its last render recorded, else by building it.
	pub async fn positions(deck: &AsyncEntity) -> Result<Vec<CardPosition>> {
		let id = deck.id();
		let routes = deck
			.world()
			.with(move |world: &mut World| -> Result<Vec<ActionNode>> {
				RouteTree::of(world, id).map(|tree| {
					Self::routes(tree).into_iter().cloned().collect()
				})
			})
			.await?;
		let mut counts = HashMap::<Entity, usize>::default();
		for node in &routes {
			let entity = node.entity;
			let known = deck
				.world()
				.with(move |world: &mut World| {
					world.get::<CardCount>(entity).map(|count| count.0)
				})
				.await;
			let count = match known {
				Some(count) => count,
				None => CardCount::build(deck, node.clone()).await?,
			};
			counts.insert(entity, count);
		}
		let routes = routes.iter().collect::<Vec<_>>();
		Self::positions_of(&routes, |node| {
			counts.get(&node.entity).copied().unwrap_or(1)
		})
		.xok()
	}
}

/// System: arrow / space / page / home / end keys step the navigator between
/// the cards of a stack, opt-in via a [`CardDeck`] marker on the in-world router.
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

	// every guard below is a clean no-op: a non-card key or Alt-arrow, no or an
	// HTTP navigator, or an unmarked router (eg the docs TUI)
	let Some((nav, (entity, navigator))) =
		nav.filter(|_| !alt).zip(navigators.single().ok())
	else {
		return;
	};
	let NavigatorTransport::InWorld { router } = navigator.transport() else {
		return;
	};
	if !decks.contains(*router) {
		return;
	}
	let (router, current) = (*router, navigator.current_url().clone());
	commands.entity(entity).queue_async(async move |entity| {
		// counting the cards may build a route never shown, so off the frame
		let positions =
			CardDeck::positions(&entity.world().entity(router)).await?;
		let current = CardPosition::of_url(&positions, &current);
		let Ok(target) = nav.resolve(&positions, &current) else {
			return Ok(());
		};
		Navigator::navigate_to(entity, target.url()).await
	});
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// A position on the route at `path`, `/` separated.
	fn at(path: &str, card: usize) -> CardPosition {
		CardPosition {
			path: path
				.split('/')
				.filter(|segment| !segment.is_empty())
				.map(SmolStr::from)
				.collect(),
			card,
		}
	}

	/// A stack of `routes`, each its path and card count.
	fn stack(routes: &[(&str, usize)]) -> Vec<CardPosition> {
		routes
			.iter()
			.flat_map(|(path, count)| (0..*count).map(|card| at(path, card)))
			.collect()
	}

	/// Where `nav` lands from `current` in `positions`, as its url.
	fn step(
		positions: &[CardPosition],
		current: CardPosition,
		nav: CardNav,
	) -> String {
		nav.resolve(positions, &current).unwrap().url().to_string()
	}

	/// A route's cards step before the next route's, clamping at the ends.
	#[beet_core::test]
	fn steps_cards_within_and_across_routes() {
		let deck = stack(&[("", 1), ("talk", 3), ("outro", 1)]);
		step(&deck, at("", 0), CardNav::Next).xpect_eq("/talk");
		step(&deck, at("talk", 0), CardNav::Next).xpect_eq("/talk?card=1");
		step(&deck, at("talk", 2), CardNav::Next).xpect_eq("/outro");
		step(&deck, at("outro", 0), CardNav::Prev).xpect_eq("/talk?card=2");
		step(&deck, at("talk", 0), CardNav::Prev).xpect_eq("/");
		// the ends clamp, a stack does not loop
		step(&deck, at("", 0), CardNav::Prev).xpect_eq("/");
		step(&deck, at("outro", 0), CardNav::Next).xpect_eq("/outro");
		step(&deck, at("talk", 1), CardNav::First).xpect_eq("/");
		step(&deck, at("talk", 1), CardNav::Last).xpect_eq("/outro");
		// a card past its route's last steps from the route's last
		step(&deck, at("talk", 9), CardNav::Next).xpect_eq("/outro");
	}

	/// `--slide=N` counts cards across routes, clamped to the last.
	#[beet_core::test]
	fn nth_card_counts_across_routes() {
		let deck = stack(&[("intro", 1), ("talk", 2), ("outro", 1)]);
		let nth = |n| CardNav::resolve_nth(&deck, n).unwrap().url().to_string();
		nth(0).xpect_eq("/intro");
		nth(2).xpect_eq("/talk?card=1");
		nth(3).xpect_eq("/outro");
		nth(99).xpect_eq("/outro");
		// a url naming a slide is that card
		CardPosition::of_url(&deck, &Url::coerce("/?slide=2"))
			.xpect_eq(at("talk", 1));
		CardPosition::of_url(&deck, &Url::coerce("/talk?card=1"))
			.xpect_eq(at("talk", 1));
	}

	/// The stack's routes are its page routes in reading order, the home
	/// card first wherever it sorts, nested folders stepped through.
	#[beet_core::test]
	fn orders_page_routes_home_first() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root = world
			.spawn((Router, NavigateHandler::default(), children![
				(
					render_action::fixed_func_route(
						"intro",
						|| rsx! { <p>"i"</p> }
					),
					PageRoute
				),
				(PathPartial::new("chapter"), children![
					(
						render_action::fixed_func_route(
							"one",
							|| rsx! { <p>"1"</p> }
						),
						PageRoute
					),
					(
						render_action::fixed_func_route(
							"two",
							|| rsx! { <p>"2"</p> }
						),
						PageRoute
					),
				]),
				(
					render_action::fixed_func_route(
						"",
						|| rsx! { <p>"home"</p> }
					),
					PageRoute
				),
				// infrastructure is no card
				render_action::fixed_func_route(
					"health",
					|| rsx! { <p>"ok"</p> }
				),
			]))
			.flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		let routes = CardDeck::routes(tree);
		CardDeck::positions_of(&routes, |_| 1)
			.iter()
			.map(|position| position.url().to_string())
			.collect::<Vec<_>>()
			.xpect_eq(vec!["/", "/chapter/one", "/chapter/two", "/intro"]);
	}
}
