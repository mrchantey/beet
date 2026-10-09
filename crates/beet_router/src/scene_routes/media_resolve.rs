//! The media resolve step: a page's images, fetched onto the tree before it
//! renders.
//!
//! The caller asks for it: the step runs in
//! [`LivePage::prepare`](crate::prelude::LivePage::prepare) when the request
//! names a standard site media ingest policy ([`MediaIngestPolicy`], the
//! `--media-ingest` render param), and only then, so a plain request fetches
//! nothing. It runs after the layouts and the `--root` cascade, over the
//! render root's subtree: each image source (an `img`'s `src`) and the page's
//! cover image (`PageMeta::social_image_url`) resolves against the page's
//! url, is fetched, and lands as an [`InlineBlob`] on the attribute's entity.
//! The attribute's value stays the link. The policy says which sources are
//! fetched: every one, the site's own alone, or none, so a target links what
//! was not fetched.
//!
//! # Fetching
//!
//! The site's own media is fetched by an ordinary in-process request to the
//! page's router, served from the repo store like any visitor's, so nothing
//! reads a filesystem and the step runs on every target; another host's is
//! fetched over the network. Every source is collected first and deduplicated
//! by its resolved link, then all are fetched through one bounded join, never
//! one await per attribute, so attributes sharing a source share one fetch and
//! one buffer. A source that fails to fetch is an error naming it and the
//! page, since a silently dropped image is a post that differs from its page.
//!
//! # Lifetime
//!
//! The bytes live on the tree for exactly its life: the page's release takes
//! back every [`InlineBlob`] it inserted, so nothing is persisted and a
//! request naming a policy resolves, renders and discards.
//! The cover image's [`InlineBlob`] sits on the render root itself, beside the
//! `PageMeta` carried there; an attribute's sits on the attribute.
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;
use std::collections::VecDeque;

/// The standard site media ingest policy: which of a page's image sources are
/// fetched into the tree, the rest being linked. The `--media-ingest` render
/// param, so a cli, a browser and a publish step set it the same way, and a
/// request naming none fetches nothing.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Default)]
pub enum MediaIngestPolicy {
	/// Fetch every source, the site's own and another host's, so each lands
	/// in the record as a blob.
	#[default]
	Upload,
	/// Fetch the site's own media and link another host's.
	Local,
	/// Fetch nothing: every source is linked.
	Link,
}

impl MediaIngestPolicy {
	/// The word the `--media-ingest` param takes for this policy.
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::Upload => "upload",
			Self::Local => "local",
			Self::Link => "link",
		}
	}

	/// Whether a source this policy governs is fetched, `own` being whether it
	/// is the site's.
	fn fetches(&self, own: bool) -> bool {
		match self {
			Self::Upload => true,
			Self::Local => own,
			Self::Link => false,
		}
	}
}

/// One source's media, fetched by the media resolve step and held on the
/// source attribute's entity (or, for the cover image, the render root) for
/// the life of the tree, so a render writes a correct [`BlobRef`] the first
/// time and a publish uploads straight from the tree it rendered.
///
/// An ordinary component, so a later step hooks in by query or observer, ie
/// an image optimiser or a size check against a PDS's blob limit.
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Component)]
pub struct InlineBlob {
	/// The bytes' content id under the raw codec, what `uploadBlob` answers.
	pub cid: Cid,
	/// The resolved source, kept for errors and deduplication.
	pub link: Url,
	/// The fetched bytes, one shared buffer every attribute naming this source
	/// holds.
	pub bytes: MediaBytes,
}

impl InlineBlob {
	/// The blob `bytes` fetched from `link` are.
	pub fn new(link: Url, bytes: MediaBytes) -> Self {
		Self {
			cid: Cid::raw(&bytes),
			link,
			bytes,
		}
	}

	/// The reference a record embeds for these bytes, computed with no
	/// network: the cid, the media type and the size.
	pub fn blob_ref(&self) -> BlobRef {
		BlobRef {
			cid: self.cid.clone(),
			mime_type: self.bytes.media_type().clone(),
			size: self.bytes.len() as u64,
		}
	}

	/// The media resolve step over the tree at `root`, a page served at
	/// `page_url` by `router`: fetch every image source under `policy` and
	/// insert an [`InlineBlob`] on each source's entity, answering the
	/// entities given one.
	pub(crate) async fn resolve(
		world: &AsyncWorld,
		router: Entity,
		page_url: Url,
		root: Entity,
		policy: MediaIngestPolicy,
	) -> Result<Vec<Entity>> {
		if policy == MediaIngestPolicy::Link {
			return Ok(Vec::new());
		}
		// collect every source first, deduplicated by its resolved link
		let homepage = world
			.with(|world: &mut World| {
				world
					.get_resource::<PackageConfig>()
					.and_then(|package| package.homepage.clone())
			})
			.await;
		let sources = world
			.with_state::<MediaSourceQuery, _>({
				let page_url = page_url.clone();
				move |query| query.collect(root, &page_url)
			})
			.await?;
		let mut fetches: Vec<(MediaSource, Vec<Entity>)> = Vec::new();
		for (entity, link) in sources {
			let source = MediaSource::new(link, homepage.as_ref());
			if !policy.fetches(source.own.is_some()) {
				continue;
			}
			match fetches
				.iter_mut()
				.find(|(seen, _)| seen.link == source.link)
			{
				Some((_, entities)) => entities.push(entity),
				None => fetches.push((source, vec![entity])),
			}
		}
		// fetch all of them at once, never one await per attribute
		let fetched = async_ext::try_join_all_bounded(
			MediaSource::IN_FLIGHT,
			fetches
				.iter()
				.map(|(source, _)| source.fetch(world, router, &page_url)),
		)
		.await?;
		world
			.with(move |world: &mut World| {
				let mut inserted = Vec::new();
				for ((source, entities), bytes) in
					fetches.into_iter().zip(fetched)
				{
					let blob = InlineBlob::new(source.link, bytes);
					for entity in entities {
						if let Ok(mut entity) = world.get_entity_mut(entity) {
							entity.insert(blob.clone());
							inserted.push(entity.id());
						}
					}
				}
				inserted
			})
			.await
			.xok()
	}
}

/// One source to fetch: its resolved link and, when it is the site's own, the
/// path its router serves it at.
struct MediaSource {
	link: Url,
	own: Option<Url>,
}

impl MediaSource {
	/// How many fetches run at once.
	const IN_FLIGHT: usize = 8;

	/// `link` classified against the site's `homepage`: a link with no host
	/// is the site's own, as is one on the homepage's host.
	fn new(link: Url, homepage: Option<&Url>) -> Self {
		let own = match link.authority() {
			None => Some(link.clone()),
			Some(authority) => homepage
				.filter(|homepage| homepage.authority() == Some(authority))
				.map(|_| Url::coerce(link.path_string())),
		};
		Self { link, own }
	}

	/// The bytes this source names: the site's own through an in-process
	/// request to `router`, another host's over the network.
	async fn fetch(
		&self,
		world: &AsyncWorld,
		router: Entity,
		page_url: &Url,
	) -> Result<MediaBytes> {
		let response =
			match &self.own {
				Some(path) => {
					world
						.entity(router)
						.exchange(Request::get(path.clone()))
						.await
				}
				None => Request::get(self.link.clone()).send().await.map_err(
					|err| {
						bevyhow!(
							"`{}` on `{page_url}` failed to fetch: {err}",
							self.link
						)
					},
				)?,
			};
		response
			.into_result()
			.await
			.map_err(|err| {
				bevyhow!(
					"`{}` on `{page_url}` failed to fetch: {err}",
					self.link
				)
			})?
			.into_media_bytes()
			.await
	}
}

/// The source attributes of a rendered tree, the walk the media resolve step
/// collects from.
#[derive(SystemParam)]
struct MediaSourceQuery<'w, 's> {
	elements: Query<'w, 's, &'static Element>,
	children: Query<'w, 's, &'static Children>,
	portals: Query<'w, 's, &'static Portal>,
	attributes: AttributeQuery<'w, 's>,
	metas: Query<'w, 's, &'static PageMeta>,
}

impl MediaSourceQuery<'_, '_> {
	/// Every image source under `root`, each resolved against `page_url`: the
	/// attributes naming one in breadth-first order, then the page's cover
	/// image, which the root itself holds.
	fn collect(
		&self,
		root: Entity,
		page_url: &Url,
	) -> Result<Vec<(Entity, Url)>> {
		let mut sources = Vec::new();
		let mut queue = VecDeque::from([root]);
		while let Some(entity) = queue.pop_front() {
			if let Ok(portal) = self.portals.get(entity) {
				queue.push_back(portal.target());
				continue;
			}
			if let Ok(element) = self.elements.get(entity) {
				for (attribute, key, value) in self.attributes.all(entity) {
					if !Self::is_image_source(element.tag(), key) {
						continue;
					}
					let Ok(text) = value.as_str() else {
						continue;
					};
					sources.push((attribute, page_url.join(Url::parse(text)?)));
				}
			}
			if let Ok(children) = self.children.get(entity) {
				queue.extend(children.iter());
			}
		}
		if let Some(cover) = self
			.metas
			.get(root)
			.ok()
			.and_then(PageMeta::social_image_url)
		{
			sources.push((root, page_url.join(cover)));
		}
		sources.xok()
	}

	/// Whether `attribute` on a `tag` element names an image the step
	/// fetches. Images alone, since the one target embedding media (Leaflet)
	/// has no video or audio block.
	fn is_image_source(tag: &str, attribute: &str) -> bool {
		tag.eq_ignore_ascii_case("img") && attribute == "src"
	}
}

/// The media every resolve and render test fetches.
#[cfg(test)]
pub(crate) mod test_fixtures {
	/// A png's header claiming `width` by `height`, which is all a size read
	/// takes from it.
	pub fn png(width: u32, height: u32) -> Vec<u8> {
		let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
		bytes.extend(13u32.to_be_bytes());
		bytes.extend(b"IHDR");
		bytes.extend(width.to_be_bytes());
		bytes.extend(height.to_be_bytes());
		bytes.extend([8, 6, 0, 0, 0, 0, 0, 0, 0]);
		bytes
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_ui::prelude::*;
	use std::sync::Arc;
	use std::sync::atomic::AtomicUsize;
	use std::sync::atomic::Ordering;

	/// A router serving `page` at `/blog/post` and the repo store's media (a
	/// 4 by 3 photo, a 2 by 1 poster, a clip) at `/media`, plus `routes`.
	async fn site<Func, B>(page: Func, routes: impl Bundle) -> (World, Entity)
	where
		Func: 'static + Send + Sync + Clone + Fn() -> B,
		B: 'static + Send + Sync + Bundle,
	{
		let store = BlobStore::temp();
		for (path, bytes) in [
			("photo.png", png(4, 3)),
			("poster.png", png(2, 1)),
			("clip.mp4", b"not really a video".to_vec()),
		] {
			store.insert(&RelPath::from(path), bytes).await.unwrap();
		}
		let mut world = syndication_world(Some("https://beet.org"));
		let router = world
			.spawn((Router::with_defaults(), store, children![
				ServeBlobs {
					prefix: "media".into(),
					cache: default(),
				}
				.into_snippet_bundle(),
				render_action::fixed_func_route("blog/post", page),
				routes,
			]))
			.flush();
		(world, router)
	}

	/// `/blog/post` under `policy`.
	fn ingesting(policy: &str) -> Request {
		Request::get("/blog/post").with_param("media-ingest", policy)
	}

	/// A route at `counted/photo.png` serving a png, counting each fetch in
	/// `fetches`.
	fn counted(fetches: &Arc<AtomicUsize>) -> impl Bundle {
		route::exchange(
			"counted/photo.png",
			exchange_ext::handler({
				let fetches = fetches.clone();
				move |_cx| {
					fetches.fetch_add(1, Ordering::SeqCst);
					Response::ok_body(png(1, 1), MediaType::Png)
				}
			}),
		)
	}

	/// The [`InlineBlob`]s the page `request` names holds once prepared,
	/// sorted by link.
	async fn prepared(
		world: &mut World,
		router: Entity,
		request: Request,
	) -> Result<Vec<InlineBlob>> {
		world
			.run_async_then(async move |world| {
				LivePage::scoped(&world.entity(router), request, async |live| {
					let mut blobs = live.inline_blobs().await;
					blobs.sort_by_key(|blob| blob.link.to_string());
					blobs.xok()
				})
				.await
			})
			.await
	}

	/// The [`InlineBlob`]s `/blog/post` holds under `policy`, sorted by link.
	async fn resolved(
		world: &mut World,
		router: Entity,
		policy: &str,
	) -> Result<Vec<InlineBlob>> {
		prepared(world, router, ingesting(policy)).await
	}

	/// The links `blobs` were fetched from.
	fn links(blobs: &[InlineBlob]) -> Vec<String> {
		blobs.iter().map(|blob| blob.link.to_string()).collect()
	}

	/// The site's own image is fetched from the repo store through the router,
	/// its blob computed from the bytes with no network.
	#[beet_core::test]
	async fn fetches_a_repo_store_image() {
		let (mut world, router) =
			site(|| rsx! { <img src="/media/photo.png"/> }, ()).await;
		let blobs = resolved(&mut world, router, "upload").await.unwrap();
		links(&blobs).xpect_eq(vec!["/media/photo.png".to_string()]);
		blobs[0]
			.blob_ref()
			.xpect_eq(BlobRef::of(&png(4, 3), MediaType::Png));
	}

	/// An absolute link on the site's own homepage is the site's own, so
	/// `local` fetches it through the router.
	#[beet_core::test]
	async fn the_homepage_host_is_the_sites_own() {
		let (mut world, router) = site(
			|| rsx! { <img src="https://beet.org/media/photo.png"/> },
			(),
		)
		.await;
		resolved(&mut world, router, "local")
			.await
			.unwrap()
			.len()
			.xpect_eq(1);
	}

	/// A relative source resolves against the page's url.
	#[beet_core::test]
	async fn resolves_a_relative_source() {
		let (mut world, router) =
			site(|| rsx! { <img src="../media/photo.png"/> }, ()).await;
		links(&resolved(&mut world, router, "upload").await.unwrap())
			.xpect_eq(vec!["/media/photo.png".to_string()]);
	}

	/// `link` fetches nothing, and a target writes its links instead.
	#[beet_core::test]
	async fn link_fetches_nothing() {
		let (mut world, router) =
			site(|| rsx! { <img src="/media/photo.png"/> }, ()).await;
		resolved(&mut world, router, "link")
			.await
			.unwrap()
			.xpect_empty();
	}

	/// A missing source is an error naming it and the page.
	#[beet_core::test]
	async fn a_missing_source_fails() {
		let (mut world, router) =
			site(|| rsx! { <img src="/media/missing.png"/> }, ()).await;
		resolved(&mut world, router, "upload")
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("/media/missing.png")
			.xpect_contains("/blog/post");
	}

	/// Images alone are fetched: a `video`'s source and its `poster` stay
	/// links.
	#[beet_core::test]
	async fn fetches_images_alone() {
		let (mut world, router) = site(
			|| {
				rsx! {
					<div>
						<video src="/media/clip.mp4" poster="/media/poster.png"/>
						<img src="/media/photo.png"/>
					</div>
				}
			},
			(),
		)
		.await;
		links(&resolved(&mut world, router, "upload").await.unwrap())
			.xpect_eq(vec!["/media/photo.png".to_string()]);
	}

	/// A request naming no policy fetches nothing, a request for Leaflet, which
	/// embeds images, included; one naming a policy fetches through the same
	/// route.
	#[beet_core::test]
	async fn a_plain_request_fetches_nothing() {
		let fetches = Arc::new(AtomicUsize::new(0));
		let (mut world, router) = site(
			|| rsx! { <img src="/counted/photo.png"/> },
			counted(&fetches),
		)
		.await;
		for accept in [
			MediaType::Html,
			#[cfg(all(feature = "dag_cbor", feature = "json"))]
			LeafletRenderer::media_type(),
		] {
			world
				.entity_mut(router)
				.exchange(Request::get("/blog/post").with_accept(accept))
				.await
				.into_result()
				.await
				.unwrap();
		}
		prepared(&mut world, router, Request::get("/blog/post"))
			.await
			.unwrap()
			.xpect_empty();
		fetches.load(Ordering::SeqCst).xpect_eq(0);
		resolved(&mut world, router, "upload")
			.await
			.unwrap()
			.len()
			.xpect_eq(1);
		fetches.load(Ordering::SeqCst).xpect_eq(1);
	}

	/// The cover image resolves with the content's images, held on the
	/// render root.
	#[beet_core::test]
	async fn resolves_the_cover_image() {
		let (mut world, router) = site(
			|| {
				(
					PageMeta {
						image_url: Some(
							Url::parse("/media/poster.png").unwrap(),
						),
						..default()
					},
					rsx! { <p>"no images"</p> },
				)
			},
			(),
		)
		.await;
		let request = ingesting("upload");
		world
			.run_async_then(async move |world| {
				LivePage::scoped(&world.entity(router), request, async |live| {
					live.cover().await.xok()
				})
				.await
			})
			.await
			.unwrap()
			.unwrap()
			.blob_ref()
			.xpect_eq(BlobRef::of(&png(2, 1), MediaType::Png));
	}

	/// Two attributes naming one source share one fetch and one buffer.
	#[beet_core::test]
	async fn a_shared_source_is_fetched_once() {
		let fetches = Arc::new(AtomicUsize::new(0));
		let (mut world, router) = site(
			|| {
				rsx! {
					<div><img src="/counted/photo.png"/><img src="/counted/photo.png"/></div>
				}
			},
			counted(&fetches),
		)
		.await;
		let blobs = resolved(&mut world, router, "upload").await.unwrap();
		fetches.load(Ordering::SeqCst).xpect_eq(1);
		blobs.len().xpect_eq(2);
		blobs[0].bytes.as_ptr().xpect_eq(blobs[1].bytes.as_ptr());
	}

	/// Every fetch runs at once rather than one after another: each held
	/// open until all three are in flight, which fetches made in turn never
	/// are, so a waterfall hangs rather than passes.
	#[beet_core::test]
	async fn fetches_in_parallel() {
		let in_flight = Arc::new(AtomicUsize::new(0));
		let (release, released) =
			beet_core::exports::async_channel::unbounded::<()>();
		let (mut world, router) = site(
			|| {
				rsx! {
					<div><img src="/slow/a.png"/><img src="/slow/b.png"/><img src="/slow/c.png"/></div>
				}
			},
			route::exchange(
				"slow/*name",
				exchange_ext::handler_async({
					let in_flight = in_flight.clone();
					move |_request| {
						let (in_flight, release, released) =
							(in_flight.clone(), release.clone(), released.clone());
						async move {
							// the third to arrive releases them all
							if in_flight.fetch_add(1, Ordering::SeqCst) + 1 == 3 {
								release.close();
							}
							released.recv().await.ok();
							Response::ok_body(png(1, 1), MediaType::Png)
						}
					}
				}),
			),
		)
		.await;
		resolved(&mut world, router, "upload")
			.await
			.unwrap()
			.len()
			.xpect_eq(3);
		in_flight.load(Ordering::SeqCst).xpect_eq(3);
	}

	/// A request naming a policy resolves, renders and leaves no fetched bytes
	/// behind, a persistent page's included.
	#[beet_core::test]
	async fn a_request_leaves_no_bytes_behind() {
		let (mut world, router) = site(
			|| rsx! { <img src="/media/photo.png"/> },
			(
				PathPartial::new("fixed"),
				FixedPage,
				rsx! { <img src="/media/photo.png"/> },
			),
		)
		.await;
		for path in ["/blog/post", "/fixed"] {
			world
				.entity_mut(router)
				.exchange(
					Request::get(path).with_param("media-ingest", "upload"),
				)
				.await
				.into_result()
				.await
				.unwrap();
			world
				.query::<&InlineBlob>()
				.iter(&world)
				.count()
				.xpect_eq(0);
		}
	}

	/// Another host's image under each policy: fetched over the network by
	/// `upload`, linked by `local` and `link`.
	#[cfg(all(feature = "http", not(target_arch = "wasm32")))]
	#[beet_core::test]
	async fn another_hosts_image_under_each_policy() {
		let store = BlobStore::temp();
		store
			.insert(&RelPath::from("remote.png"), png(8, 8))
			.await
			.unwrap();
		let (mut server, on_spawn) =
			HttpServer::new_test(HttpServer::start_mini_with_tcp);
		// leave the process-global loopback port to whoever owns it
		server.canonical = false;
		let remote = format!("{}/media/remote.png", server.local_url());
		std::thread::spawn(move || {
			App::new()
				.add_plugins((MinimalPlugins, RouterPlugin))
				.spawn((server, on_spawn, store, children![(
					Router::default(),
					children![
						ServeBlobs {
							prefix: "media".into(),
							cache: default(),
						}
						.into_snippet_bundle()
					]
				)]))
				.run();
		});
		let page = {
			let remote = remote.clone();
			move || {
				(
					Element::new("img"),
					Attribute::bundle("src", remote.clone()),
				)
			}
		};
		let (mut world, router) = site(page, ()).await;
		links(&resolved(&mut world, router, "upload").await.unwrap())
			.xpect_eq(vec![remote.clone()]);
		resolved(&mut world, router, "local")
			.await
			.unwrap()
			.xpect_empty();
		resolved(&mut world, router, "link")
			.await
			.unwrap()
			.xpect_empty();
	}
}
