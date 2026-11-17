//! An article's comments: the replies to its announcement on Bluesky, read
//! live from the AppView.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
#[cfg(feature = "atproto")]
use beet_ui::prelude::*;
#[cfg(feature = "atproto")]
use beet_ui::style::WhiteSpace;
#[cfg(feature = "atproto")]
use beet_ui::style::common_props::WhiteSpaceProp;

/// The replies to the page's announcement, each with its author, avatar, time
/// and text, nested as the thread nests, oldest first, then a
/// [`StandardSiteReplyLink`]. Nothing at all for a page with no announcement.
///
/// The page is the one the [`RequestContextStack`] names, its announcement
/// the `bsky_post` its publication's [`StandardSiteIndex`] remembers, and the
/// thread is read from the nearest [`AppView`] above its route, the public one
/// where none is declared. Every render reads the AppView, so place it in a
/// [`ServerIsland`] behind a short edge cache, the link its fallback:
///
/// ```html
/// <ServerIsland name="comments" {CacheHeaders{edge_max_age: "60s"}}>
/// 	<StandardSiteComments/>
/// 	<StandardSiteReplyLink slot="fallback"/>
/// </ServerIsland>
/// ```
///
/// The read is async content, shaped like [`VisitCount`]: a failed read, or a
/// build without the `atproto` feature, rides [`TemplateError`], so the
/// island route fails and the placeholder keeps its fallback.
#[template(system)]
pub fn StandardSiteComments(
	stack: Res<RequestContextStack>,
	standard_site: StandardSiteQuery,
) -> Result<OnSpawn> {
	let cx = stack.current();
	match StandardSiteReplyLink::announcement(&standard_site, cx) {
		None => OnSpawn::new(|_| {}).xok(),
		Some(announcement) => {
			StandardSiteComments::fetch(cx.route(), announcement)
		}
	}
}

/// The link to reply to the page's announcement on Bluesky, nothing for a
/// page with no announcement: the fallback of a [`StandardSiteComments`]
/// island, and its last line once it swaps in.
#[template(system)]
pub fn StandardSiteReplyLink(
	stack: Res<RequestContextStack>,
	standard_site: StandardSiteQuery,
) -> impl Bundle {
	let link =
		StandardSiteReplyLink::announcement(&standard_site, stack.current())
			.map(|announcement| StandardSiteReplyLink::link(&announcement.uri));
	rsx! { {link} }
}

impl StandardSiteReplyLink {
	/// The announcement of the page `cx` renders, as its publication's index
	/// remembers it.
	fn announcement(
		standard_site: &StandardSiteQuery,
		cx: &RequestContext,
	) -> Option<StrongRef> {
		standard_site
			.document(&RelPath::new(cx.current_path()))
			.and_then(|document| document.bsky_post.clone())
	}

	/// The paragraph linking the announcement at `uri`.
	fn link(uri: &AtUri) -> Snippet {
		let href = FeedPost::web_url(uri);
		rsx! { <p><a href=href>"Reply on Bluesky"</a></p> }
	}
}

/// A build without the AppView client cannot read the thread.
#[cfg(not(feature = "atproto"))]
impl StandardSiteComments {
	fn fetch(_route: Entity, announcement: StrongRef) -> Result<OnSpawn> {
		bevybail!(
			"`<StandardSiteComments/>` reads the replies to {} from the \
			 Bluesky AppView, which this build lacks: enable the `atproto` \
			 feature",
			announcement.uri
		)
	}
}

#[cfg(feature = "atproto")]
impl StandardSiteComments {
	/// How many levels of replies a render shows, the AppView's own default.
	const DEPTH: u16 = 6;

	/// Read the thread of `announcement` once spawned, parked as async
	/// content.
	fn fetch(route: Entity, announcement: StrongRef) -> Result<OnSpawn> {
		OnSpawn::new(move |entity| {
			TemplatePending::register_fetch(
				entity,
				PendingKind::Structural,
				"<StandardSiteComments/>",
				move |entity, guard| {
					Self::read(entity, route, announcement, guard)
				},
			)
		})
		.xok()
	}

	/// Read the thread through the [`AppView`] above `route`, then spawn the
	/// comments under `entity` and resolve the dependency.
	async fn read(
		entity: AsyncEntity,
		route: Entity,
		announcement: StrongRef,
		guard: PendingGuard,
	) {
		let world = entity.world();
		let app_view = world
			.with_state::<AncestorQuery<&AppView>, _>(move |app_views| {
				app_views.get(route).cloned().unwrap_or_default()
			})
			.await;
		let thread = app_view
			.get_post_thread(&announcement.uri, Self::DEPTH, 0)
			.await;
		let parent = entity.id();
		world
			.with(move |world: &mut World| {
				// a failure rides `TemplateError` like any build failure
				let comments = thread
					.and_then(|thread| Self::render(&announcement, &thread));
				match (comments, world.get_entity_mut(parent)) {
					(Ok(comments), Ok(mut entity)) => {
						entity.with_child(comments);
					}
					(Err(err), Ok(mut entity)) => {
						entity.insert(TemplateError::new(err));
					}
					// the page was released before the read landed
					(_, Err(_)) => {}
				}
				guard.resolve(world);
			})
			.await;
	}

	/// The comments section of `thread`, the thread of `announcement`.
	///
	/// # Errors
	/// Errors when the AppView no longer shows the announcement, ie it was
	/// deleted.
	fn render(
		announcement: &StrongRef,
		thread: &PostThread,
	) -> Result<Snippet> {
		let Some(root) = thread.post() else {
			bevybail!(
				"the AppView shows no announcement at {}: the index names a \
				 post that is gone",
				announcement.uri
			);
		};
		let replies = Self::replies(root);
		let link = StandardSiteReplyLink::link(&announcement.uri);
		match replies {
			None => rsx! { <section>{link}</section> },
			Some(replies) => rsx! {
				<section>
					<h2>"Comments"</h2>
					{replies}
					{link}
				</section>
			},
		}
		.xok()
	}

	/// `post`'s replies as a list, each holding its own, `None` when it has
	/// none.
	fn replies(post: &ThreadViewPost) -> Option<Snippet> {
		let replies = post.replies_chronological();
		if replies.is_empty() {
			return None;
		}
		let items = replies.into_iter().map(Self::comment).collect::<Vec<_>>();
		Some(rsx! { <ol>{items}</ol> })
	}

	/// One reply: its byline (avatar, author, time linking the post), its
	/// text, then its own replies.
	fn comment(reply: &ThreadViewPost) -> Snippet {
		let post = &reply.post;
		let author = &post.author;
		let profile = format!("{}/profile/{}", FeedPost::WEB_APP, author.did);
		let avatar = author.avatar.clone().map(|src| {
			rsx! { <img src=src alt="" width="24" height="24" {Classes::new([classes::SHAPE_FULL])}/> }
		});
		let created = post.record.created_at.to_string();
		let date = Timestamp::parse_rfc3339(&created)
			.map(|timestamp| Date::from(timestamp).format_long())
			.unwrap_or_else(|| created.clone());
		let label = author.label().to_string();
		let link = FeedPost::web_url(&post.uri);
		let text = Self::text(&post.record);
		let replies = Self::replies(reply);
		rsx! {
			<li>
				<p {Classes::new([classes::TEXT_LABEL_MEDIUM])}>
					{avatar}" "<a href=profile>{label}</a>" · "
					<a href=link><time datetime=created>{date}</time></a>
				</p>
				<p {inline_class![(WhiteSpaceProp, WhiteSpace::PreWrap)]}>{text}</p>
				{replies}
			</li>
		}
	}

	/// A post's text, its links, mentions and hashtags live.
	fn text(post: &FeedPost) -> Vec<Snippet> {
		post.segments()
			.into_iter()
			.map(|(text, feature)| {
				let text = text.to_string();
				match feature.and_then(FacetFeature::href) {
					Some(href) => rsx! { <a href=href>{text}</a> },
					None => rsx! { {text} },
				}
			})
			.collect()
	}
}

#[cfg(test)]
mod test {
	use super::super::test_fixtures::*;
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// The announcement the recorded thread hangs off.
	fn announcement() -> StrongRef {
		StrongRef::new(
			AtUri::parse(
				"at://did:plc:vgxy56shhs3t3b2mu2avizjf/app.bsky.feed.post/3mw72aaeuj22p",
			)
			.unwrap(),
			Cid::parse(
				"bafyreihzukpzbnzrlnlrg4bweuercgc3r5tz6f37e6qr3n34rneslivaza",
			)
			.unwrap(),
		)
	}

	/// An AppView answering `getPostThread` with the recorded thread when
	/// `recorded`, else answering nothing. One server for every case, on its
	/// own thread and port, its loop paced so it never spins a core.
	#[cfg(all(
		feature = "atproto",
		feature = "http",
		not(target_arch = "wasm32")
	))]
	fn app_view(recorded: bool) -> AppView {
		static ORIGIN: std::sync::OnceLock<String> = std::sync::OnceLock::new();
		let origin = ORIGIN.get_or_init(|| {
			let store = BlobStore::temp();
			async_ext::block_on(store.insert(
				&RelPath::from("recorded/xrpc/app.bsky.feed.getPostThread"),
				include_str!("fixtures/post_thread.json"),
			))
			.unwrap();
			let (mut server, on_spawn) =
				HttpServer::new_test(HttpServer::start_mini_with_tcp);
			// leave the process-global loopback port to whoever owns it
			server.canonical = false;
			let origin = server.local_url();
			std::thread::spawn(move || {
				App::new()
					.add_plugins((
						MinimalPlugins.set(
							bevy::app::ScheduleRunnerPlugin::run_loop(
								Duration::from_millis(1),
							),
						),
						RouterPlugin,
					))
					.spawn((server, on_spawn, store, children![(
						Router::default(),
						children![
							ServeBlobs {
								prefix: "appview".into(),
								cache: default(),
							}
							.into_snippet_bundle()
						]
					)]))
					.run();
			});
			origin
		});
		match recorded {
			true => AppView::new(format!("{origin}/appview/recorded")),
			false => AppView::new(format!("{origin}/appview/missing")),
		}
	}

	/// The fixture site under an article layout holding the comments island,
	/// its index announcing `blog/full-stack-bevy` and no other post, reading
	/// threads through `app_view`.
	async fn site(app_view: impl Bundle) -> (World, Entity) {
		let store = BlobStore::temp();
		let mut index = harvest_index();
		index.documents[0].bsky_post = Some(announcement());
		write_index(&store, &index).await;
		let mut world = syndication_world(Some("https://beet.org"));
		world
			.get_resource_or_init::<BsxTemplateRegistry>()
			.insert_source(
				"CommentsLayout",
				r#"<html><body><main><Slot/></main>
				<ServerIsland name="comments">
					<StandardSiteComments/>
					<StandardSiteReplyLink slot="fallback"/>
				</ServerIsland>
				</body></html>"#,
			)
			.unwrap();
		let root = spawn_syndication_router_with(
			&mut world,
			(store, Layout::new("CommentsLayout"), app_view),
			rsx! {
				<StandardSitePub path="blog" name="The Full Moon Harvest"/>
				<ServerIslands/>
			},
		);
		TemplatePending::settle_owned(&mut world).await;
		(world, root)
	}

	async fn get(
		world: &mut World,
		root: Entity,
		path: &str,
		accept: MediaType,
	) -> Response {
		world
			.entity_mut(root)
			.exchange(Request::get(path).with_accept(accept))
			.await
	}

	const REPLY_LINK: &str = r#"<a href="https://bsky.app/profile/did:plc:vgxy56shhs3t3b2mu2avizjf/post/3mw72aaeuj22p">Reply on Bluesky</a>"#;

	/// The whole page leaves the island a placeholder, the reply link its
	/// fallback, reading no reply.
	#[beet_core::test]
	async fn page_carries_the_fallback() {
		let (mut world, root) = site(()).await;
		get(&mut world, root, "blog/full-stack-bevy", MediaType::Html)
			.await
			.unwrap_str()
			.await
			.xpect_contains(&format!(
				r#"<beet-island name="comments"><p>{REPLY_LINK}</p></beet-island>"#
			))
			.xnot()
			.xpect_contains("the first reply");
	}

	/// A page with no announcement renders nothing, fallback and island
	/// alike.
	#[beet_core::test]
	async fn no_announcement_renders_nothing() {
		let (mut world, root) = site(()).await;
		get(&mut world, root, "blog/ecs-router", MediaType::Html)
			.await
			.unwrap_str()
			.await
			.xpect_contains(r#"<beet-island name="comments"></beet-island>"#);
		get(
			&mut world,
			root,
			"_server-islands/blog/ecs-router/comments",
			MediaType::Html,
		)
		.await
		.unwrap_str()
		.await
		.xnot()
		.xpect_contains("Bluesky");
	}

	/// The island answers the replies, oldest first and nested as the thread
	/// nests, their links and mentions live, a missing reply skipped, then
	/// the reply link.
	#[cfg(all(
		feature = "atproto",
		feature = "http",
		not(target_arch = "wasm32")
	))]
	#[beet_core::test]
	async fn island_renders_the_replies() {
		let (mut world, root) = site(app_view(true)).await;
		let html = get(
			&mut world,
			root,
			"_server-islands/blog/full-stack-bevy/comments",
			MediaType::Html,
		)
		.await
		.unwrap_str()
		.await;
		html.clone()
			// a post keeps its own line breaks
			.xpect_contains("white-space: pre-wrap")
			.xpect_contains("<h2>Comments</h2>")
			.xpect_contains(r#"src="https://cdn.bsky.app/img/avatar/plain/did:plc:2wco2e2ybeg2lwpfx3kqjsuv/alice@jpeg""#)
			.xpect_contains(r#"<a href="https://bsky.app/profile/did:plc:2wco2e2ybeg2lwpfx3kqjsuv">Alice</a>"#)
			.xpect_contains(r#"<time datetime="2025-07-12T09:30:00.000Z">12 July 2025</time>"#)
			.xpect_contains(r#"<a href="https://bsky.app/profile/did:plc:vgxy56shhs3t3b2mu2avizjf">@beet.org</a>"#)
			.xpect_contains(r#"<a href="https://bevy.org">https://bevy.org</a>"#)
			// a profile with no display name is named by its handle
			.xpect_contains(">bob.example.com</a>")
			.xpect_contains(REPLY_LINK)
			.xnot()
			.xpect_contains("3mw72aaeuj23d");
		let position = |text: &str| html.find(text).unwrap();
		// oldest first, the nested answer inside the reply it answers
		(position("the first reply") < position("the second reply"))
			.xpect_true();
		(position("the second reply") < position("a nested answer"))
			.xpect_true();
		html[position("the second reply")..]
			.find("<ol>")
			.xpect_some();
	}

	/// A text render resolves the island inline, the replies in the page.
	#[cfg(all(
		feature = "atproto",
		feature = "http",
		not(target_arch = "wasm32")
	))]
	#[beet_core::test]
	async fn text_renders_inline() {
		let (mut world, root) = site(app_view(true)).await;
		get(&mut world, root, "blog/full-stack-bevy", MediaType::Text)
			.await
			.unwrap_str()
			.await
			.xpect_contains("the first reply")
			.xpect_contains("a nested answer")
			.xpect_contains("Reply on Bluesky");
	}

	/// An AppView that cannot answer fails the island, so the placeholder
	/// keeps its fallback.
	#[cfg(all(
		feature = "atproto",
		feature = "http",
		not(target_arch = "wasm32")
	))]
	#[beet_core::test]
	async fn a_failed_read_fails_the_island() {
		let (mut world, root) = site(app_view(false)).await;
		get(
			&mut world,
			root,
			"_server-islands/blog/full-stack-bevy/comments",
			MediaType::Html,
		)
		.await
		.into_result()
		.await
		.unwrap_err()
		.to_string()
		.xpect_contains("reading the thread of");
	}

	/// A build without the AppView client fails the island naming the
	/// feature.
	#[cfg(not(feature = "atproto"))]
	#[beet_core::test]
	async fn needs_atproto() {
		let (mut world, root) = site(()).await;
		get(
			&mut world,
			root,
			"_server-islands/blog/full-stack-bevy/comments",
			MediaType::Html,
		)
		.await
		.into_result()
		.await
		.unwrap_err()
		.to_string()
		.xpect_contains("`atproto`");
	}
}
