//! The island step of a page's preparation: every [`ServerIsland`] on the
//! page, its layouts included, resolved as the caller asks.
//!
//! [`LivePage::prepare`] runs it after the layouts and before the `--root`
//! cascade. Which islands resolve is the caller's, never a request param: the
//! http page handler asks for placeholders when it negotiated html, the
//! island route ([`ServerIslands`]) names the one island it answers, which
//! becomes the render root, and every other caller (a live surface, a scoped
//! render, a non-html response) resolves every island inline.
//!
//! An island's content builds inside its page's [`RequestContext`], so a
//! widget in it reads the page off [`RequestContextStack`] as a layout widget
//! does, and content that parks a [`TemplatePending`] dependency (a store
//! read, a network fetch) is awaited before the root resolves. Everything the
//! step spawns sits under its island marked [`IslandResolution`]: the next
//! resolution of the same island replaces it, and the page's release takes it
//! back, so a persistent tree keeps nothing between requests.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;
use beet_ui::style::CssBuilder;
use beet_ui::style::CssTokenMap;
use beet_ui::style::FormatVariables;
use beet_ui::style::StyleQuery;

/// Which of a page's server islands its preparation resolves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum IslandScope {
	/// Every island's content inline, the default for anything not cached at
	/// the edge as html.
	Inline,
	/// Every island a placeholder, its content never built: the http page
	/// handler's html.
	Placeholders,
	/// Only the named island, built inline and made the render root: the
	/// island route. A page with no island of that name is a 404.
	Only(SmolStr),
}

/// Marks an entity the island step spawned under a [`ServerIsland`], replaced
/// by the island's next resolution and taken back on its page's release.
#[derive(Debug, Default, Clone, Copy, Component)]
pub(crate) struct IslandResolution;

/// What the island step put on a page.
#[derive(Debug, Default)]
pub(crate) struct ResolvedIslands {
	/// Every entity the step spawned, released with the page.
	pub spawned: Vec<Entity>,
	/// The island the render roots at, under [`IslandScope::Only`].
	pub root: Option<Entity>,
}

impl IslandScope {
	/// Resolve the islands of the tree at `rendered` in `cx`, the page's
	/// request context. A failure takes back whatever the step spawned.
	pub(crate) async fn resolve(
		self,
		world: &AsyncWorld,
		rendered: Entity,
		cx: RequestContext,
	) -> Result<ResolvedIslands> {
		// build the placeholders or the content, synchronously
		let (resolved, built) = world
			.with(move |world: &mut World| {
				let mut resolved = ResolvedIslands::default();
				let built = self.build(world, rendered, &cx, &mut resolved);
				(resolved, built.map(|built| (self, built)))
			})
			.await;
		let (scope, built) = match built {
			Ok(built) => built,
			Err(err) => return Self::discard(world, resolved, err).await,
		};
		// content arriving asynchronously lands before the root resolves
		Self::settle(world, built.iter().map(|(_, content)| *content)).await;
		let (resolved, finished) = world
			.with(move |world: &mut World| {
				let mut resolved = resolved;
				let finished = scope.finish(world, &built, &mut resolved);
				(resolved, finished)
			})
			.await;
		match finished {
			Ok(()) => resolved.xok(),
			Err(err) => Self::discard(world, resolved, err).await,
		}
	}

	/// Build what this scope resolves for each island under `rendered`,
	/// answering each built island beside the root its content built into.
	fn build(
		&self,
		world: &mut World,
		rendered: Entity,
		cx: &RequestContext,
		resolved: &mut ResolvedIslands,
	) -> Result<Vec<(Entity, Entity)>> {
		let islands = Self::islands(world, rendered)?;
		match self {
			Self::Placeholders => {
				if !islands.is_empty() {
					Self::assert_island_route(world, cx.router())?;
				}
				for (entity, island) in islands {
					Self::clear(world, entity);
					Self::spawn_placeholder(
						world, entity, &island, cx, resolved,
					)?;
				}
				Vec::new().xok()
			}
			Self::Only(name) => {
				let (entity, island) = islands
					.into_iter()
					.find(|(_, island)| &island.name == name)
					.ok_or_else(|| {
						HttpError::new(
							StatusCode::NOT_FOUND,
							format!("this page has no server island `{name}`"),
						)
					})?;
				resolved.root = Some(entity);
				Self::clear(world, entity);
				let content =
					Self::spawn_content(world, entity, &island, cx, resolved)?;
				vec![(entity, content)].xok()
			}
			Self::Inline => islands
				.into_iter()
				.map(|(entity, island)| {
					Self::clear(world, entity);
					Self::spawn_content(world, entity, &island, cx, resolved)
						.map(|content| (entity, content))
				})
				.collect(),
		}
	}

	/// Check each built island's content, and lead the named island of
	/// [`IslandScope::Only`] with the stylesheet its own subtree selects, so a
	/// swap is styled on a process that never built it before.
	fn finish(
		&self,
		world: &mut World,
		built: &[(Entity, Entity)],
		resolved: &mut ResolvedIslands,
	) -> Result {
		for (island, content) in built {
			Self::assert_content(world, *island, *content)?;
		}
		if let (Self::Only(_), Some(island)) = (self, resolved.root)
			&& let Some(style) = Self::spawn_fragment_style(world, island)?
		{
			resolved.spawned.push(style);
		}
		Ok(())
	}

	/// Every island under `rendered`, through each [`Portal`], refusing a name
	/// that is not one path segment and a name two islands share.
	fn islands(
		world: &mut World,
		rendered: Entity,
	) -> Result<Vec<(Entity, ServerIsland)>> {
		let islands = world
			.with_state::<(RenderTreeQuery, Query<&ServerIsland>), _>(
				|(tree, islands)| {
					tree.iter_descendants_inclusive(rendered)
						.filter_map(|entity| {
							islands
								.get(entity)
								.ok()
								.map(|island| (entity, island.clone()))
						})
						.collect::<Vec<_>>()
				},
			);
		let mut names = HashSet::<SmolStr>::default();
		for (_, island) in &islands {
			ServerIsland::assert_name(&island.name)?;
			if !names.insert(island.name.clone()) {
				bevybail!(
					"two server islands on one page are named `{}`: a name \
					 addresses its island, so each must be unique per page",
					island.name
				);
			}
		}
		islands.xok()
	}

	/// Refuse a placeholder whose script would fetch nothing: the router
	/// serving the page must declare [`ServerIslands`].
	fn assert_island_route(world: &World, router: Entity) -> Result {
		let path = vec![
			SmolStr::new_static(ServerIsland::PREFIX),
			SmolStr::new_static("island"),
		];
		let declared = world
			.get::<RouteTree>(router)
			.and_then(|tree| tree.find(&path))
			.is_some_and(|node| {
				world.entity(node.entity).contains::<ServerIslandsRoute>()
			});
		match declared {
			true => Ok(()),
			false => bevybail!(
				"this page carries a server island but its router declares no \
				 `<ServerIslands/>`, so the placeholder's script would fetch \
				 nothing: declare `<ServerIslands/>` at the router's root"
			),
		}
	}

	/// Despawn an earlier resolution of `island`, ie a persistent tree's from
	/// a previous request or surface.
	fn clear(world: &mut World, island: Entity) {
		let stale = world
			.entity(island)
			.get::<Children>()
			.into_iter()
			.flat_map(|children| children.iter())
			.filter(|child| world.entity(*child).contains::<IslandResolution>())
			.collect::<Vec<_>>();
		for entity in stale {
			world.entity_mut(entity).despawn();
		}
	}

	/// Spawn under `entity` a child holding `island`'s content, built inside
	/// the page's request context, answering the child.
	fn spawn_content(
		world: &mut World,
		entity: Entity,
		island: &ServerIsland,
		cx: &RequestContext,
		resolved: &mut ResolvedIslands,
	) -> Result<Entity> {
		let content = world.spawn((IslandResolution, ChildOf(entity))).id();
		resolved.spawned.push(content);
		RequestContextStack::scoped(world, cx.clone(), |world| {
			island.content.build(&mut world.entity_mut(content))
		})?;
		content.xok()
	}

	/// Spawn under `entity` the placeholder: a wrapper naming the island with
	/// the fallback inside it, then the script that swaps the island in.
	fn spawn_placeholder(
		world: &mut World,
		entity: Entity,
		island: &ServerIsland,
		cx: &RequestContext,
		resolved: &mut ResolvedIslands,
	) -> Result {
		let name = island.name.to_string();
		let wrapper = world
			.spawn((IslandResolution, ChildOf(entity), rsx! {
				<div data-beet-island=name/>
			}))
			.id();
		resolved.spawned.push(wrapper);
		if !island.fallback.is_empty() {
			let fallback = world.spawn(ChildOf(wrapper)).id();
			RequestContextStack::scoped(world, cx.clone(), |world| {
				island.fallback.build(&mut world.entity_mut(fallback))
			})?;
		}
		let script = world
			.spawn((IslandResolution, ChildOf(entity), rsx! {
				<script>{ServerIsland::SWAP_SCRIPT}</script>
			}))
			.id();
		resolved.spawned.push(script);
		Ok(())
	}

	/// Refuse island content carrying a `<script>`, which a swap inserts
	/// inert, or another island, which nothing would resolve.
	fn assert_content(
		world: &mut World,
		island: Entity,
		content: Entity,
	) -> Result {
		let found = world.with_state::<(
			RenderTreeQuery,
			Query<&Element>,
			Query<(), With<ServerIsland>>,
		), _>(|(tree, elements, islands)| {
			tree.iter_descendants_inclusive(content).find_map(|entity| {
				if islands.contains(entity) {
					Some("another server island")
				} else if elements.get(entity).is_ok_and(|element| {
					element.tag().eq_ignore_ascii_case("script")
				}) {
					Some("a `<script>`, which a swapped in island never runs")
				} else {
					None
				}
			})
		});
		match found {
			None => Ok(()),
			Some(found) => {
				let name = world
					.get::<ServerIsland>(island)
					.map(|island| island.name.clone())
					.unwrap_or_default();
				bevybail!("server island `{name}` contains {found}")
			}
		}
	}

	/// Spawn the `<style>` leading `island`: the rules its subtree's classes
	/// select, named as the page's stylesheet names them. `None` in a world
	/// with no rule set or no rule to carry.
	fn spawn_fragment_style(
		world: &mut World,
		island: Entity,
	) -> Result<Option<Entity>> {
		if !world.contains_resource::<RuleSet>()
			|| !world.contains_resource::<CssTokenMap>()
		{
			return Ok(None);
		}
		let css = world.with_state::<(RenderTreeQuery, StyleQuery), _>(
			|(tree, style)| {
				let classes = tree
					.iter_descendants_inclusive(island)
					.filter_map(|entity| style.elements().get(entity).ok())
					.flat_map(|view| view.iter_classes().collect::<Vec<_>>())
					.collect::<HashSet<_>>();
				style.build_css_selecting(
					&CssBuilder::default()
						.with_minify(true)
						.with_format_variables(FormatVariables::short()),
					&classes,
				)
			},
		)?;
		if css.trim().is_empty() {
			return Ok(None);
		}
		let style = world
			.spawn((IslandResolution, rsx! { <style>{css}</style> }))
			.id();
		world.entity_mut(island).insert_children(0, &[style]);
		Some(style).xok()
	}

	/// Await every [`TemplatePending`] set on the `content` roots, the async
	/// content the island step builds before the root resolves. A dropped
	/// guard resolves through the sweep, so this cannot hang on a lost one.
	async fn settle(world: &AsyncWorld, content: impl Iterator<Item = Entity>) {
		let content = content.collect::<Vec<_>>();
		if content.is_empty() {
			return;
		}
		loop {
			let content = content.clone();
			let pending = world
				.with(move |world: &mut World| {
					world.flush();
					TemplatePending::sweep_dropped(world);
					content.iter().any(|entity| {
						world
							.get::<TemplatePending>(*entity)
							.is_some_and(|pending| !pending.is_empty())
					})
				})
				.await;
			if !pending {
				return;
			}
			// a dependency task bridges the world between its awaits, so give
			// it a sync point before counting again (see `TemplatePending::settle`)
			async_ext::yield_now().await;
			world.with(|_| ()).await;
		}
	}

	/// Take back what the step spawned and fail with `err`.
	async fn discard(
		world: &AsyncWorld,
		resolved: ResolvedIslands,
		err: BevyError,
	) -> Result<ResolvedIslands> {
		world
			.with(move |world: &mut World| {
				for entity in resolved.spawned {
					if let Ok(entity) = world.get_entity_mut(entity) {
						entity.despawn();
					}
				}
			})
			.await;
		Err(err)
	}
}
