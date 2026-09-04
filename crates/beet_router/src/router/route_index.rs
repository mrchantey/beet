//! The generated page index: a route's child pages rendered as a linked list.
//!
//! [`RouteIndex`] is the listing half of frontmatter-driven content. Where
//! [`RouteSidebar`](crate::prelude::RouteSidebar) collects the whole
//! [`RouteTree`] into a nav rail, this collects the *current* route's children
//! into the body of the page, so a blog index is a `<RouteIndex reverse="true"/>`
//! rather than a hand-maintained list that drifts from the posts it points at.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_ui::prelude::*;

/// The current route's child pages, rendered as a linked index: a numbered
/// heading, an `AUTHOR · DATE` eyebrow, and the page's description.
///
/// Reads this request's tree off the [`RequestContext::router`] handle (see
/// [`RouteSidebar`](crate::prelude::RouteSidebar) for why the handle rather than
/// a walk) and lists the children of the node the request matched, so an
/// `index.md` under `blog/` lists the posts beside it. Every field comes from
/// each page's [`ArticleMeta`], ie its markdown frontmatter: a child with no
/// frontmatter has nothing to list and is skipped, as is one that is not a
/// [`PageRoute`], and a draft is skipped in production exactly as static export
/// drops it.
///
/// Entries are ordered like the nav — frontmatter `order`, which a numbered
/// filename fills in ([`ArticleMeta::with_file_defaults`]), then natural order
/// by path — and numbered `#1..#N` by their place in that ascending order.
/// `reverse` flips the *display* only, so a blog reads newest first while post
/// `#1` stays post `#1`.
///
/// Registered by name (see [`RouterPlugin`](crate::prelude::RouterPlugin)), so a
/// markup page places it with `<RouteIndex reverse="true"/>`. Builds inside a
/// route render (it reads [`RequestContext`], which the content build pushes
/// just as the layout middleware does).
#[template(system)]
pub fn RouteIndex(
	/// List the entries newest first, ie descending order. The `#N` numbering is
	/// unaffected, being the entry's place in ascending order.
	#[prop]
	reverse: bool,
	stack: Res<RequestContextStack>,
	trees: Query<&RouteTree>,
	metas: Query<&ArticleMeta>,
) -> impl Bundle {
	let cx = stack.current();
	// drafts are listed everywhere but production, matching `StaticExport`
	let is_prod = BootstrapConfig::get().is_prod();
	let current = SmolPath::new(cx.current_path());
	let mut entries: Vec<(SmolPath, ArticleMeta)> = trees
		.get(cx.router())
		.ok()
		.and_then(|tree| tree.find_subtree(&current.segments()))
		.map(|subtree| {
			subtree
				.children
				.iter()
				.filter_map(|child| Some((child, child.node()?)))
				.filter(|(_, node)| node.is_page_route)
				.filter_map(|(child, node)| {
					let meta = metas.get(node.entity).ok()?;
					(!(is_prod && meta.draft))
						.then(|| (child.path.annotated_path(), meta.clone()))
				})
				.collect()
		})
		.unwrap_or_default();
	entries.sort_by(|(path_a, meta_a), (path_b, meta_b)| {
		let order = |meta: &ArticleMeta| meta.sidebar.order.unwrap_or(u32::MAX);
		order(meta_a)
			.cmp(&order(meta_b))
			.then_with(|| natural_cmp(path_a.as_str(), path_b.as_str()))
	});
	// numbered in ascending order, then flipped for display, so `reverse`
	// renumbers nothing
	let mut items: Vec<(usize, SmolPath, ArticleMeta)> = entries
		.into_iter()
		.enumerate()
		.map(|(idx, (path, meta))| (idx + 1, path, meta))
		.collect();
	if reverse {
		items.reverse();
	}
	let items: Vec<Snippet> = items
		.into_iter()
		.enumerate()
		.map(|(idx, (number, path, meta))| {
			index_entry(number, &path, &meta, idx > 0)
		})
		.collect();
	rsx! { {items} }
}

/// One index entry: the `<hr/>` separating it from the entry above (every entry
/// but the first), its numbered heading linking to the page, then whichever of
/// the eyebrow and description the frontmatter supplied.
fn index_entry(
	number: usize,
	path: &SmolPath,
	meta: &ArticleMeta,
	separator: bool,
) -> Snippet {
	let rule = match separator {
		true => rsx! { <hr/> },
		false => Snippet::from_bundle(()),
	};
	let href = path.with_leading_slash();
	let heading = format!(
		"#{number} — {}",
		meta.title
			.as_deref()
			.unwrap_or_else(|| path.last_segment().unwrap_or_default())
	);
	let eyebrow = entry_eyebrow(meta);
	let description = entry_description(meta);
	rsx! {
		{rule}
		<h4><a href=href>{heading}</a></h4>
		{eyebrow}
		{description}
	}
}

/// The `AUTHOR · DATE` eyebrow line, or nothing when the frontmatter declares
/// neither: an entry without one renders no empty paragraph.
fn entry_eyebrow(meta: &ArticleMeta) -> Snippet {
	let text = [
		meta.author.as_deref().map(str::to_uppercase),
		meta.created
			.map(|created| created.format_long_date().to_uppercase()),
	]
	.into_iter()
	.flatten()
	.collect::<Vec<_>>()
	.join(" · ");
	if text.is_empty() {
		return Snippet::from_bundle(());
	}
	rsx! { <p {Classes::new([classes::TEXT_EYEBROW])}>{text}</p> }
}

/// The entry's summary, or nothing when the frontmatter has no `description`.
fn entry_description(meta: &ArticleMeta) -> Snippet {
	let Some(text) = meta.description.clone() else {
		return Snippet::from_bundle(());
	};
	rsx! { <p>{text}</p> }
}

#[cfg(test)]
mod test {
	use super::*;
	use beet_net::prelude::*;

	/// A page route at `path` carrying the frontmatter an index entry reads.
	/// The rendered body is irrelevant: collection keys off the tree and the
	/// metadata, so every fixture route shares one.
	fn post(path: &str, meta: ArticleMeta) -> impl Bundle {
		(
			render_action::fixed_func_route(path, || rsx! { <p>"post"</p> }),
			PageRoute,
			meta,
		)
	}

	/// Frontmatter as a numbered blog post would declare it.
	fn meta(title: &str, created: &str, order: u32) -> ArticleMeta {
		ArticleMeta {
			title: Some(title.into()),
			description: Some(format!("about {title}")),
			author: Some("Pete Hayman".into()),
			created: Timestamp::parse_date(created),
			sidebar: SidebarInfo {
				order: Some(order),
				..default()
			},
			..default()
		}
	}

	/// A router world holding a `blog` index and three posts beside it, with the
	/// request context a render would push: the request is for `blog`, so the
	/// index lists that node's children.
	fn index_world() -> World {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root = world
			.spawn(children![(PathPartial::new("blog"), children![
				post(
					"full-stack-bevy",
					meta("Full Stack Bevy", "2025-07-11", 1)
				),
				post("ecs-router", meta("ECS Router", "2025-08-09", 2)),
				post("declarative-state", {
					let mut meta = meta("Declarative State", "2025-11-03", 3);
					meta.draft = true;
					meta
				}),
			])])
			.flush();
		world
			.resource_mut::<RequestContextStack>()
			.push(RequestContext::new(
				RequestParts::get("blog"),
				root,
				root,
				root,
			));
		world
	}

	/// Render `<RouteIndex reverse=..>` in `world` to HTML.
	fn render(world: &mut World, reverse: bool) -> String {
		let entity = world
			.spawn_template(rsx! { <RouteIndex reverse=reverse/> })
			.unwrap()
			.id();
		HtmlRenderer::new()
			.render(&mut RenderContext::new(entity, world))
			.unwrap()
			.to_string()
	}

	#[beet_core::test]
	fn renders_entries_in_order() {
		render(&mut index_world(), false).xpect_snapshot();
	}

	/// `reverse` flips the display without renumbering: post `#1` is still `#1`,
	/// it is merely listed last.
	#[beet_core::test]
	fn reverse_lists_newest_first() {
		render(&mut index_world(), true).xpect_snapshot();
	}

	/// End to end through the real page path: a discovered `blog/index.md`
	/// placing `<RouteIndex/>` lists the posts discovered beside it, at the urls
	/// their `slug` frontmatter names.
	///
	/// The posts declare their metadata as frontmatter, which only the
	/// `markdown_parser` build reads; without it a post carries no `ArticleMeta`
	/// and the index has nothing to list.
	#[cfg(feature = "markdown_parser")]
	#[beet_core::test]
	async fn lists_discovered_posts() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let store = BlobStore::temp();
		for (path, content) in [
			(
				"blog/index.bsx",
				r#"<Fragment {ArticleMeta{title: "Blog"}}><h1>Blog</h1><RouteIndex reverse="true"/></Fragment>"#,
			),
			(
				"blog/1-full-stack-bevy.md",
				"+++\ntitle = \"Full Stack Bevy\"\nslug = \"full-stack-bevy\"\ndescription = \"the first one\"\ncreated = \"2025-07-11\"\nauthor = \"Pete Hayman\"\n+++\n\n# One",
			),
			(
				"blog/2-ecs-router.md",
				"+++\ntitle = \"ECS Router\"\nslug = \"ecs-router\"\ndescription = \"the second one\"\ncreated = \"2025-08-09\"\nauthor = \"Pete Hayman\"\n+++\n\n# Two",
			),
		] {
			store.insert(&SmolPath::from(path), content).await.unwrap();
		}
		let root = world
			.spawn((store, Router, children![RoutesDir::default()]))
			.flush();
		AsyncRunner::settle_async_tasks(&mut world).await;

		world
			.entity_mut(root)
			.exchange(
				Request::get("blog")
					.with_header::<header::Accept>(vec![MediaType::Html]),
			)
			.await
			.unwrap_str()
			.await
			.xpect_snapshot();
	}
}
