//! What a render of a deck's route shows: one card, its notes hidden.
//!
//! A card root is a route's content root when its document holds no
//! `article data-card`, else each such article. Its source is its visible
//! front, a markdown thematic break (`---`, lowered to an `<hr>` child),
//! then the back-of-card notes, ie speaker notes, which never show. The
//! [`CardDeck`]'s render middleware runs for every render of a route beneath
//! it, the one-shot HTML render and the live terminal page alike: it sets
//! `hidden` on each card root's first `hr` child and every sibling after
//! it, and on every card article but the one the request's `card` names, so
//! a writer, which reads no `hidden` as a hidden slide for a note, still
//! finds the notes after a deck has rendered. A render naming `slide`
//! shows the stack's `slide`th card in place of the route's own.
//!
//! The marker gates by ancestry, through the route's middleware, so an
//! `<hr>` in ordinary beet content stays a horizontal rule.

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

/// On a deck's route: how many cards its last render or count held, a
/// route of no card articles one, so stepping the stack builds a route only
/// once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deref, Component)]
pub struct CardCount(pub usize);

/// The traversal of a document's cards.
#[derive(SystemParam)]
pub(crate) struct CardQuery<'w, 's> {
	children: Query<'w, 's, &'static Children>,
	elements: ElementQuery<'w, 's>,
}

impl CardQuery<'_, '_> {
	/// The card articles under `root`, outermost only, in document order.
	pub fn cards(&self, root: Entity) -> Vec<Entity> {
		let mut out = Vec::new();
		self.collect_cards(root, &mut out);
		out
	}

	fn collect_cards(&self, entity: Entity, out: &mut Vec<Entity>) {
		for child in self.children_of(entity) {
			match self.is_card(child) {
				true => out.push(child),
				false => self.collect_cards(child, out),
			}
		}
	}

	fn children_of(&self, entity: Entity) -> Vec<Entity> {
		self.children
			.get(entity)
			.map(|children| children.to_vec())
			.unwrap_or_default()
	}

	/// Whether `entity` is an `article data-card`.
	pub fn is_card(&self, entity: Entity) -> bool {
		self.elements.get(entity).is_ok_and(|view| {
			view.tag() == "article" && view.attribute("data-card").is_some()
		})
	}

	/// Whether `entity` carries `hidden`.
	pub fn is_hidden(&self, entity: Entity) -> bool {
		self.elements
			.get(entity)
			.is_ok_and(|view| view.attribute("hidden").is_some())
	}

	/// A card root's notes: its first `hr` child and every sibling after.
	pub fn notes(&self, root: Entity) -> Vec<Entity> {
		let children = self.children_of(root);
		children
			.iter()
			.position(|child| {
				self.elements
					.get(*child)
					.is_ok_and(|view| view.tag() == "hr")
			})
			.map(|start| children[start..].to_vec())
			.unwrap_or_default()
	}

	/// The cards a reader of `root` steps through: each card article not
	/// hidden, a document holding none one card, `root` itself.
	pub fn shown(&self, root: Entity) -> Vec<Entity> {
		let cards = self.cards(root);
		match cards.is_empty() {
			true => vec![root],
			false => cards
				.into_iter()
				.filter(|card| !self.is_hidden(*card))
				.collect(),
		}
	}
}

impl CardCount {
	/// Builds `node`'s content under `deck`, counts its shown cards, records
	/// the count on the route and releases the content.
	pub(crate) async fn build(
		deck: &AsyncEntity,
		node: ActionNode,
	) -> Result<usize> {
		let route = node.entity;
		let request = Request::get(CardPosition::first(&node.path).url());
		let page = LivePage::build_content(deck, node, request).await?;
		deck.world()
			.with(move |world: &mut World| {
				let count = world.with_state::<CardQuery, _>(|cards| {
					cards.shown(page).len()
				});
				LivePage::despawn_content(world, page);
				if let Ok(mut route) = world.get_entity_mut(route) {
					route.insert(CardCount(count));
				}
				count
			})
			.await
			.xok()
	}
}

impl CardDeck {
	/// Pushes the deck's render middleware, which shows one card of each
	/// render and hides its notes, and its request middleware, which answers
	/// a request for the home card accepting a slide deck with the whole
	/// stack.
	pub(crate) fn on_add(mut world: DeferredWorld, cx: HookContext) {
		let deck = cx.entity;
		world.commands().entity(deck).queue(
			move |mut entity: EntityWorldMut| {
				entity
					.get_mut_or_default::<MiddlewareList<RequestParts, Entity>>(
					)
					.add(
						move |parts: RequestParts,
						      next: Next<RequestParts, Entity>| async move {
							let world = next.world().clone();
							let route = next.id();
							let page = next.call(parts.clone()).await?;
							let params = parts
								.parse_params::<CardParams>()
								.unwrap_or_default();
							// a slide shows the stack's card in place of the route's own
							let (route, card) = match params.slide {
								Some(slide) => {
									let deck = world.entity(deck);
									let positions =
										CardDeck::positions(&deck).await?;
									let position = CardNav::resolve_nth(
										&positions, slide,
									)?;
									let node = LivePage::find_route(
										&deck,
										position.path.clone(),
									)
									.await?
									.ok_or_else(|| {
										bevyhow!(
											"no route at {}",
											position.url()
										)
									})?;
									let substitute = node.entity;
									let built = LivePage::build_content(
										&deck,
										node,
										Request::get(position.url()),
									)
									.await?;
									world
										.with(move |world: &mut World| {
											Self::substitute(world, page, built)
										})
										.await?;
									(substitute, position.card)
								}
								None => {
									(route, params.card.unwrap_or_default())
								}
							};
							world
								.with(move |world: &mut World| {
									Self::show(world, route, page, card)
								})
								.await?;
							Ok::<_, BevyError>(page)
						},
					);
				entity
					.get_mut_or_default::<MiddlewareList<Request, Response>>()
					.add(
						move |request: Request,
						      next: Next<Request, Response>| async move {
							let deck = next.world().entity(deck);
							match CardDeck::exports(&deck, &request).await? {
								true => CardDeck::export(&deck, request).await,
								false => next.call(request).await,
							}
						},
					);
			},
		);
	}

	/// The content of `page`, its layouts aside, replaced by `built`'s: its
	/// children and its metadata, `built` released.
	fn substitute(world: &mut World, page: Entity, built: Entity) -> Result {
		let content = LayoutContent::terminal(world, page)?;
		let old = world
			.entity(content)
			.get::<Children>()
			.map(|children| children.to_vec())
			.unwrap_or_default();
		for child in old {
			world.entity_mut(child).despawn();
		}
		let new = world
			.entity(built)
			.get::<Children>()
			.map(|children| children.to_vec())
			.unwrap_or_default();
		world.entity_mut(content).add_children(&new);
		match world.get::<DocMeta>(built).cloned() {
			Some(meta) => world.entity_mut(content).insert(meta),
			None => world.entity_mut(content).remove::<DocMeta>(),
		};
		LivePage::despawn_content(world, built);
		Ok(())
	}

	/// Shows card `card` of `page`'s content, the route's other cards and
	/// every card's notes hidden, and records the route's count.
	fn show(
		world: &mut World,
		route: Entity,
		page: Entity,
		card: usize,
	) -> Result {
		let content = LayoutContent::terminal(world, page)?;
		let (shown, hide) = world.with_state::<CardQuery, _>(|cards| {
			let shown = cards.shown(content);
			let selected = card.min(shown.len().saturating_sub(1));
			let mut hide = shown
				.iter()
				.enumerate()
				.filter(|(index, card)| *index != selected && **card != content)
				.map(|(_, card)| *card)
				.collect::<Vec<_>>();
			let roots = match cards.cards(content) {
				cards if cards.is_empty() => vec![content],
				cards => cards,
			};
			for root in roots {
				hide.extend(cards.notes(root));
			}
			(shown.len(), hide)
		});
		for entity in hide {
			Self::hide(world, entity);
		}
		if let Ok(mut route) = world.get_entity_mut(route) {
			route.insert(CardCount(shown));
		}
		Ok(())
	}

	/// Sets `hidden` on `entity`, once.
	fn hide(world: &mut World, entity: Entity) {
		let hidden =
			world.with_state::<CardQuery, _>(|cards| cards.is_hidden(entity));
		if !hidden {
			world.spawn((
				AttributeOf::new(entity),
				Attribute::new("hidden"),
				Value::Null,
			));
		}
	}

	/// Whether `request` asks the deck for the whole stack as a slide deck:
	/// it names the home card and accepts a deck by name, where the Office
	/// formats are linked to write one.
	async fn exports(deck: &AsyncEntity, request: &Request) -> Result<bool> {
		if !cfg!(feature = "ooxml")
			|| !request.parts().accept().contains(&MediaType::Pptx)
		{
			return Ok(false);
		}
		let (id, path) = (deck.id(), request.path().clone());
		deck.world()
			.with(move |world: &mut World| -> Result<bool> {
				let tree = RouteTree::of(world, id)?;
				let home = CardDeck::routes(tree)
					.first()
					.map(|node| {
						node.path
							.iter()
							.map(|segment| SmolStr::from(segment.name()))
							.collect::<Vec<_>>()
					})
					.unwrap_or_default();
				Ok(path.is_empty() || path == home)
			})
			.await
	}

	/// The whole stack as one slide deck: the home card's page, its layouts
	/// around it so every card resolves the looks the deck shows it in, its
	/// content replaced by every card route's in card order, each route's
	/// media fetched against its own url, then written as the request
	/// accepts.
	pub(crate) async fn export(
		deck: &AsyncEntity,
		request: Request,
	) -> Result<Response> {
		let id = deck.id();
		let routes = deck
			.world()
			.with(move |world: &mut World| -> Result<Vec<ActionNode>> {
				RouteTree::of(world, id).map(|tree| {
					Self::routes(tree).into_iter().cloned().collect()
				})
			})
			.await?;
		let home = routes
			.first()
			.cloned()
			.ok_or_else(|| bevyhow!("card stack has no cards"))?;
		let named = RenderParams::of(request.parts())?.media_ingest;
		let policy = named
			.or_else(|| MediaIngestPolicy::implied(request.parts()))
			.unwrap_or(MediaIngestPolicy::Link);
		// the home content is replaced, so its own media is never fetched
		let layout_request = match named {
			Some(_) => request,
			None => request
				.with_param("media-ingest", MediaIngestPolicy::Link.as_str()),
		};
		let live = LivePage::prepare_route(deck, home, layout_request).await?;
		live.release_after(async |live: LivePage| {
			let world = live.world().clone();
			let content = live.content().await?;
			world
				.with(move |world: &mut World| {
					let children = world
						.get::<Children>(content)
						.map(|children| children.to_vec())
						.unwrap_or_default();
					for child in children {
						world.entity_mut(child).despawn();
					}
				})
				.await;
			for node in routes {
				let url = CardPosition::first(&node.path).url();
				let parts = Request::get(url.clone()).parts().clone();
				let page =
					LivePage::build_content(deck, node, Request::get(url))
						.await?;
				let fetched =
					MediaResolve::resolve(&world, id, &parts, page, policy)
						.await;
				world
					.with(move |world: &mut World| {
						Self::adopt(world, content, page)
					})
					.await;
				fetched?;
			}
			live.render(&MediaType::Pptx).await
		})
		.await?
		.xmap(|bytes| Response::ok().with_media(bytes))
		.xok()
	}

	/// Moves `page`'s cards under `content`, a page of no card articles one
	/// card of its content, then releases `page`.
	fn adopt(world: &mut World, content: Entity, page: Entity) {
		let cards = world.with_state::<CardQuery, _>(|cards| cards.cards(page));
		let cards = match cards.is_empty() {
			false => cards,
			true => {
				let children = world
					.entity(page)
					.get::<Children>()
					.map(|children| children.to_vec())
					.unwrap_or_default();
				let card = world.spawn(Element::new("article")).id();
				world.spawn((
					AttributeOf::new(card),
					Attribute::new("data-card"),
					Value::Null,
				));
				world.entity_mut(card).add_children(&children);
				vec![card]
			}
		};
		world.entity_mut(content).add_children(&cards);
		LivePage::despawn_content(world, page);
	}
}

#[cfg(test)]
#[cfg(all(feature = "ooxml", feature = "markdown"))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_scene::prelude::*;
	use beet_ui::prelude::*;

	/// A synthetic two slide deck, each slide's notes after its `hr`,
	/// written by the deck writer.
	fn talk() -> Vec<u8> {
		let mut world =
			(TemplatePlugin, DocumentPlugin, OoxmlPlugin).into_world();
		let root = world.spawn_empty().id();
		ParseContext::new(
			&mut world.entity_mut(root),
			&MediaBytes::new_html(
				"<article data-card><h1>Talk one</h1><p>First slide.</p><hr/><p>Talk notes.</p></article>\
				 <article data-card><h1>Talk two</h1><p>Second slide.</p><hr/><p>More notes.</p></article>",
			),
		).parse()
		.unwrap();
		DocRenderers::render(
			&mut world,
			root,
			&RequestParts::default().with_accept(MediaType::Pptx),
		)
		.unwrap()
		.bytes()
		.to_vec()
	}

	/// A store of a markdown route of one card showing the deck's own
	/// picture, a markdown route of two card articles, a deck and the picture.
	async fn store() -> BlobStore {
		let store = BlobStore::new(InMemoryBlobStore::new());
		for (path, body) in [
			(
				"01-intro.md",
				"# Intro\n\nWelcome.\n\n![A picture](/media/pic.png)\n\n---\n\nIntro notes.\n"
					.as_bytes()
					.to_vec(),
			),
			("pic.png", crate::scene_routes::png(4, 2)),
			(
				"02-pair.md",
				"<article data-card>\n\n# First\n\nOne.\n\n---\n\nFirst notes.\n\n</article>\n\n\
				 <article data-card>\n\n# Second\n\nTwo.\n\n---\n\nSecond notes.\n\n</article>\n"
					.as_bytes()
					.to_vec(),
			),
			("talk.pptx", talk()),
		] {
			store.insert(&RelPath::from(path), body).await.unwrap();
		}
		store
	}

	/// A router over [`store`] serving its pictures under `media`, a stack of
	/// cards when `deck`.
	async fn router(world: &mut World, deck: bool) -> Entity {
		let store = store().await;
		let serve = || {
			ServeBlobs {
				prefix: "media".into(),
				cache: default(),
			}
			.into_snippet_bundle()
		};
		let root = match deck {
			true => world.spawn((store, Router, CardDeck, children![
				RoutesDir::default(),
				serve()
			])),
			false => world.spawn((store, Router, children![
				RoutesDir::default(),
				serve()
			])),
		}
		.flush();
		AsyncRunner::settle_async_tasks(world).await;
		root
	}

	async fn get(
		world: &mut World,
		root: Entity,
		url: &str,
		accept: MediaType,
	) -> Response {
		world
			.entity_mut(root)
			.exchange(
				Request::get(url).with_header::<header::Accept>(vec![accept]),
			)
			.await
	}

	/// The stack steps every card across its routes: a route of no card
	/// articles one card, a route of several a card each.
	#[beet_core::test]
	async fn counts_cards_across_routes() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root = router(&mut world, true).await;
		let deck = world.entity(root).id();
		let positions = world
			.run_async_then(move |world| async move {
				CardDeck::positions(&world.entity(deck)).await
			})
			.await
			.unwrap();
		positions
			.iter()
			.map(|position| position.url().to_string())
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"/01-intro",
				"/02-pair",
				"/02-pair?card=1",
				"/talk",
				"/talk?card=1",
			]);
	}

	/// Over http `talk?card=1` answers the second slide alone, every other
	/// card and every card's notes hidden, which markdown leaves out.
	#[beet_core::test]
	async fn answers_one_card() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root = router(&mut world, true).await;
		get(&mut world, root, "talk?card=1", MediaType::Markdown)
			.await
			.unwrap_str()
			.await
			.xpect_contains("Talk two")
			.xpect_contains("Second slide.")
			.xnot()
			.xpect_contains("Talk one")
			.xnot()
			.xpect_contains("notes");
		let html = get(&mut world, root, "talk?card=1", MediaType::Html)
			.await
			.unwrap_str()
			.await;
		html.clone().xpect_contains("<article data-card hidden>");
		html.xpect_contains("<hr hidden />");
		// a markdown file's cards step the same way
		get(&mut world, root, "02-pair", MediaType::Markdown)
			.await
			.unwrap_str()
			.await
			.xpect_contains("One.")
			.xnot()
			.xpect_contains("Two.");
		// and `slide` counts across routes
		get(&mut world, root, "01-intro?slide=2", MediaType::Markdown)
			.await
			.unwrap_str()
			.await
			.xpect_contains("Two.")
			.xnot()
			.xpect_contains("Welcome.");
	}

	/// Without a deck each document renders whole, its notes and every card.
	#[beet_core::test]
	async fn renders_whole_without_a_deck() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root = router(&mut world, false).await;
		get(&mut world, root, "talk", MediaType::Markdown)
			.await
			.unwrap_str()
			.await
			.xpect_contains("Talk one")
			.xpect_contains("Talk two")
			.xpect_contains("Talk notes.");
	}

	/// A request for the home card accepting a slide deck answers the whole
	/// stack, a slide per card in order, each card's notes its own and the
	/// deck's own pictures inside it.
	#[beet_core::test]
	async fn exports_the_stack() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root = router(&mut world, true).await;
		let bytes = get(&mut world, root, "/", MediaType::Pptx)
			.await
			.into_media_bytes()
			.await
			.unwrap();
		bytes.media_type().clone().xpect_eq(MediaType::Pptx);
		let read = world.spawn_empty().id();
		ParseContext::new(&mut world.entity_mut(read), &bytes)
			.parse()
			.unwrap();
		let html = HtmlRenderer::new()
			.render(&mut RenderContext::new(
				&mut world,
				read,
				&RequestParts::default(),
			))
			.unwrap()
			.to_string();
		html.matches("<article data-card>").count().xpect_eq(5);
		let order = ["Intro", "First", "Second", "Talk one", "Talk two"]
			.map(|title| html.find(title).unwrap());
		order.windows(2).all(|pair| pair[0] < pair[1]).xpect_true();
		html.clone()
			.xpect_contains("<hr /><p>First notes.</p>")
			.xpect_contains("<hr /><p>More notes.</p>");
		// the deck's own picture is fetched into the file
		html.xpect_contains("src=\"data:image/png;base64,");
	}

	/// In the terminal the arrows step every card in order across the three
	/// routes, and `--slide=N` opens on the Nth.
	#[cfg(feature = "tui")]
	#[beet_core::test]
	async fn steps_the_stack_in_the_terminal() {
		use crate::navigate::navigator::test::drive_until;
		use bevy::input::ButtonState;
		use bevy::input::keyboard::Key;
		use bevy::input::keyboard::KeyCode;
		use bevy::input::keyboard::KeyboardInput;
		use bevy::math::UVec2;

		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			RouterPlugin,
			RealtimeParsePlugin,
			PageHostPlugin,
			NavigatorPlugin,
		));
		let root = router(app.world_mut(), true).await;
		let host = app
			.world_mut()
			.spawn((
				PageHost::bundle(UVec2::new(60, 12)),
				// the opening route a launch's `--slide=3` makes
				Navigator::in_world(root, "/?slide=3"),
			))
			.id();
		drive_until(&mut app, host, "Talk one").await;
		let frame = |app: &App| {
			app.world()
				.get::<DoubleBuffer>(host)
				.map(|buffer| buffer.current_buffer().render_plain())
				.unwrap_or_default()
		};
		// one card shows, never its notes nor the route's other card
		frame(&app)
			.xnot()
			.xpect_contains("Talk notes")
			.xnot()
			.xpect_contains("Talk two");
		for (code, needle, url) in [
			(KeyCode::Home, "Welcome.", "/01-intro"),
			(KeyCode::ArrowRight, "One.", "/02-pair"),
			(KeyCode::ArrowRight, "Two.", "/02-pair?card=1"),
			(KeyCode::ArrowRight, "First slide.", "/talk"),
			(KeyCode::ArrowRight, "Second slide.", "/talk?card=1"),
			(KeyCode::ArrowLeft, "First slide.", "/talk"),
		] {
			app.world_mut().write_message(KeyboardInput {
				key_code: code,
				logical_key: Key::ArrowRight,
				state: ButtonState::Pressed,
				text: None,
				repeat: false,
				window: host,
			});
			drive_until(&mut app, host, needle).await;
			app.world()
				.get::<Navigator>(host)
				.unwrap()
				.current_url()
				.to_string()
				.xpect_eq(url);
		}
	}
}
