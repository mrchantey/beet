//! Which part of a page a render answers: the `--root` render param and the
//! one cascade that resolves it.
//!
//! `?root=main`, `--root=main` and a field on an in-process request are the
//! same switch, a param on every scene route's [`RenderParams`], so `--help`
//! documents it and each value is its own cache key. It chooses the tree a
//! target renders, never how, so it applies to every registered target. With
//! no `--root` a render answers the whole tree, document chrome and all.
//!
//! # The cascade
//!
//! `main` and `content` are one cascade over the rendered tree, layouts
//! included, the first rung that matches being the root a target renders:
//!
//! 1. the first `<main>` element in breadth-first order;
//! 2. the entity a [`LayoutContent`] names, the route content a layout
//!    transcludes;
//! 3. the first `<body>` element in breadth-first order;
//! 4. the whole tree.
//!
//! `main` starts at the first rung, `content` at the second. Breadth-first
//! means several `<main>` elements are not an error: the shallowest, then the
//! earliest, wins. The walk follows a [`Portal`] into the tree it transcludes,
//! exactly as a render does.
//!
//! # Main and content
//!
//! A layout's `<main>` holds more than the route's content: `ArticleLayout`
//! renders `ArticleHeader` (the `<h1>` title, the byline and the companion
//! video, all from `PageMeta`) inside it. A `main` render keeps that header,
//! which is what a reader asking for the main content generally wants; a
//! `content` render carries nothing a layout contributed, which is what every
//! syndication target wants, since the record already carries the title and
//! dates. So no target strips a duplicated title itself.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;
use std::collections::VecDeque;

/// Which part of a page a render answers, the `--root` render param, its
/// absence being the whole tree. See the [module docs](self) for the cascade
/// `main` and `content` resolve through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum RenderRoot {
	/// The page's main content: its first `<main>`, else the route content a
	/// layout transcludes, else its `<body>`, else the whole tree.
	Main,
	/// The route's own content, nothing a layout contributed: the content a
	/// layout transcludes, else the page's `<body>`, else the whole tree.
	Content,
}

/// The params every scene route's render reads, on every scene route by
/// construction so `--help` lists them.
#[derive(Debug, Default, Clone, Reflect)]
pub struct RenderParams {
	/// The part of the page to render: `main` (its main content) or
	/// `content` (the route's own content, nothing a layout contributed),
	/// unset for the whole page.
	pub root: Option<RenderRoot>,
	/// The standard site media ingest policy: when set, the page's images are
	/// fetched onto the tree before it renders, `upload` every source,
	/// `local` the site's own, `link` none; unset fetches nothing.
	pub media_ingest: Option<MediaIngestPolicy>,
}

impl RenderParams {
	/// The render params `parts` carries.
	pub fn of(parts: &RequestParts) -> Result<Self> { parts.parse_params() }
}

/// The cascade [`RenderRoot`] resolves through, over a rendered tree.
#[derive(SystemParam)]
pub struct RenderRootQuery<'w, 's> {
	elements: Query<'w, 's, &'static Element>,
	children: Query<'w, 's, &'static Children>,
	portals: Query<'w, 's, &'static Portal>,
	layout_contents: Query<'w, 's, &'static LayoutContent>,
}

impl RenderRootQuery<'_, '_> {
	/// The entity a render as `root` starts from, `rendered` being the whole
	/// tree and the answer to no `root`.
	pub fn resolve(
		&self,
		rendered: Entity,
		root: Option<RenderRoot>,
	) -> Entity {
		let main = || self.first_element(rendered, "main");
		let content = || self.layout_content(rendered);
		let body = || self.first_element(rendered, "body");
		match root {
			None => None,
			Some(RenderRoot::Main) => main().or_else(content).or_else(body),
			Some(RenderRoot::Content) => content().or_else(body),
		}
		.unwrap_or(rendered)
	}

	/// The route content the layout at `rendered` transcludes. Every layout in
	/// a chain links the page itself rather than the layout it wraps (see
	/// [`LayoutContent`]), so one hop reaches it.
	fn layout_content(&self, rendered: Entity) -> Option<Entity> {
		self.layout_contents
			.get(rendered)
			.ok()
			.map(LayoutContent::get)
	}

	/// The first `<{tag}>` element under `start` in breadth-first order,
	/// stepping through each [`Portal`] into the tree it transcludes.
	fn first_element(&self, start: Entity, tag: &str) -> Option<Entity> {
		let mut queue = VecDeque::from([start]);
		while let Some(entity) = queue.pop_front() {
			if let Ok(portal) = self.portals.get(entity) {
				queue.push_back(portal.target());
				continue;
			}
			if self
				.elements
				.get(entity)
				.is_ok_and(|element| element.tag().eq_ignore_ascii_case(tag))
			{
				return Some(entity);
			}
			if let Ok(children) = self.children.get(entity) {
				queue.extend(children.iter());
			}
		}
		None
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_ui::prelude::*;

	/// The tag of the entity `root` resolves to over `tree`.
	fn resolve(tree: impl Bundle, root: Option<RenderRoot>) -> String {
		let mut world = world_ext::ui_world();
		let rendered = world.spawn(tree).id();
		let resolved = world.with_state::<RenderRootQuery, _>(|query| {
			query.resolve(rendered, root)
		});
		match resolved == rendered {
			true => "whole".to_string(),
			false => world
				.get::<Element>(resolved)
				.map(|element| element.tag().to_string())
				.unwrap_or_else(|| "content".to_string()),
		}
	}

	#[beet_core::test]
	fn none_is_the_whole_tree() {
		resolve(rsx! { <html><body><main/></body></html> }, None)
			.xpect_eq("whole");
	}

	/// The shallowest `<main>` wins, then the earliest.
	#[beet_core::test]
	fn main_is_the_shallowest_main() {
		let tree = rsx! {
			<html><body>
				<div><main id="deep"/></div>
				<main id="shallow"/>
			</body></html>
		};
		let mut world = world_ext::ui_world();
		let rendered = world.spawn(tree).id();
		let main = world.with_state::<RenderRootQuery, _>(|query| {
			query.resolve(rendered, Some(RenderRoot::Main))
		});
		world
			.with_state::<AttributeQuery, _>(|attributes| {
				attributes
					.all(main)
					.iter()
					.map(|(_, _, value)| value.to_string())
					.collect::<Vec<_>>()
			})
			.xpect_eq(vec!["shallow".to_string()]);
	}

	/// With no `<main>` and no layout, `main` and `content` fall to the body,
	/// then to the whole tree.
	#[beet_core::test]
	fn falls_to_the_body_then_the_tree() {
		resolve(
			rsx! { <html><body><p/></body></html> },
			Some(RenderRoot::Main),
		)
		.xpect_eq("body");
		resolve(
			rsx! { <html><body><p/></body></html> },
			Some(RenderRoot::Content),
		)
		.xpect_eq("body");
		resolve(rsx! { <div><p/></div> }, Some(RenderRoot::Main))
			.xpect_eq("whole");
	}

	/// A layout's transcluded content is the second rung: `content` takes it
	/// over the layout's `<main>`, `main` takes the `<main>`.
	#[beet_core::test]
	fn content_is_what_a_layout_transcludes() {
		let mut world = world_ext::ui_world();
		let content = world
			.spawn((Element::new("article"), children![Value::from("body")]))
			.id();
		let layout = world
			.spawn((
				LayoutContent::new(content),
				Element::new("html"),
				children![(Element::new("body"), children![(
					Element::new("main"),
					children![Portal::new(content)]
				)])],
			))
			.id();
		let resolve = |world: &mut World, root| {
			world.with_state::<RenderRootQuery, _>(|query| {
				query.resolve(layout, root)
			})
		};
		resolve(&mut world, Some(RenderRoot::Content)).xpect_eq(content);
		let main = resolve(&mut world, Some(RenderRoot::Main));
		world.get::<Element>(main).unwrap().tag().xpect_eq("main");
	}

	/// A `<main>` inside transcluded content is found through the portal.
	#[beet_core::test]
	fn walks_through_portals() {
		let mut world = world_ext::ui_world();
		let content = world.spawn(rsx! { <section><main/></section> }).id();
		let shell = world
			.spawn((Element::new("div"), children![Portal::new(content)]))
			.id();
		let main = world.with_state::<RenderRootQuery, _>(|query| {
			query.resolve(shell, Some(RenderRoot::Main))
		});
		world.get::<Element>(main).unwrap().tag().xpect_eq("main");
	}

	/// `--help` lists `--root` and the words it takes on a scene route.
	#[beet_core::test]
	async fn help_lists_the_root_param() {
		let mut world = syndication_world(None);
		let router = world
			.spawn((Router::with_defaults(), children![page(
				"post",
				default()
			)]))
			.flush();
		world
			.entity_mut(router)
			.exchange(Request::get("/?help").with_accept(MediaType::Text))
			.await
			.unwrap_str()
			.await
			.xpect_contains("root")
			.xpect_contains("one of: main, content");
	}
}
