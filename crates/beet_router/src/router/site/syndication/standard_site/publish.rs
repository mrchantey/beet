//! A page published as a standard site document: the document rendered off
//! one live page, and the blobs it references uploaded off the same tree.
//!
//! The publish step works inside [`LivePage::scoped`]: it prepares each page
//! once with [`StandardSitePub::request`], so the tree is the route's own
//! content and its images are fetched onto it ([`InlineBlob`]) under the
//! publication's media ingest policy, renders the document
//! ([`StandardSiteDocument::render`]), uploads each blob the record references
//! from the tree ([`LivePage::upload_blob_refs`]), then writes the record,
//! all before the scope releases the tree. The render writes nothing to a
//! repo, so a dry run renders exactly what a publish writes, and the upload
//! and the write land minutes apart in one run, well inside a PDS's grace for
//! an unreferenced blob (several hours recommended, one hour the floor).
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

impl StandardSitePub {
	/// The request a document of this publication renders the page at `path`
	/// with: the route's own content, nothing a layout contributed, its images
	/// resolved under the publication's standard site media ingest policy,
	/// named explicitly since a request naming none fetches nothing.
	pub fn request(&self, path: &RelPath) -> Request {
		Request::get(path.with_leading_slash())
			.with_param("root", "content")
			.with_param("media-ingest", self.media_ingest.as_str())
	}
}

impl StandardSiteDocument {
	/// The document the listed page at `path` publishes as within
	/// `publication`, written to the publication record at `site`, rendered
	/// off `live`, the page prepared with the publication's
	/// [`request`](StandardSitePub::request): what
	/// [`from_page`](Self::from_page) fills from `meta`, the plain text render
	/// as `textContent`, the `content` format's render wrapped under its NSID,
	/// and the cover image the media resolve step fetched as `coverImage`.
	/// The contributors and the announcement are the publish step's.
	pub async fn render(
		publication: &StandardSitePub,
		site: AtUri,
		path: &RelPath,
		meta: &PageMeta,
		live: &LivePage,
		formats: &StandardSiteContentFormats,
	) -> Result<Self> {
		let mut document = Self::from_page(publication, site, path, meta)?;
		if publication.text_content {
			document.text_content = live
				.render(&MediaType::Text)
				.await?
				.as_utf8()?
				.trim_end()
				.to_string()
				.xmap(Some);
		}
		if let Some(nsid) = &publication.content {
			let bytes = live.render(formats.media_type(nsid)?).await?;
			document.content = Some(formats.member(nsid, &bytes)?);
		}
		document.cover_image = live.cover().await.map(|cover| cover.blob_ref());
		document.xok()
	}
}

impl LivePage {
	/// Upload to `pds` every blob `record` references, each from the
	/// [`InlineBlob`] on this page's tree with its cid, answering them. A
	/// referenced cid no [`InlineBlob`] holds is an error naming it, since a
	/// record pointing at a blob its repo never received is refused by the
	/// repo, and uploading the same bytes again is a no-op.
	pub async fn upload_blob_refs(
		&self,
		pds: &Pds,
		record: &AtprotoValue,
	) -> Result<Vec<BlobRef>> {
		let held = self.inline_blobs().await;
		let mut uploaded = Vec::new();
		for blob_ref in record.blob_refs() {
			let blob = held
				.iter()
				.find(|blob| blob.cid == blob_ref.cid)
				.ok_or_else(|| {
					bevyhow!(
						"the record references blob {}, which no source on the \
						 page it was rendered off holds",
						blob_ref.cid
					)
				})?;
			uploaded.push(
				pds.upload_blob(
					blob.bytes.bytes().clone(),
					blob.bytes.media_type().clone(),
				)
				.await?,
			);
		}
		uploaded.xok()
	}
}

#[cfg(test)]
mod test {
	use super::super::test_fixtures::*;
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_ui::prelude::*;

	/// The fixture post's frontmatter: a cover image, a title and a date.
	fn meta() -> PageMeta {
		PageMeta {
			image_url: Some(Url::parse("/media/cover.png").unwrap()),
			..post("Full Stack Bevy", "2025-07-11")
		}
	}

	/// A router serving the fixture post at `blog/full-stack-bevy`, an image
	/// in its body and a cover in its frontmatter, the media in its repo
	/// store.
	async fn site() -> (World, Entity) {
		let store = BlobStore::temp();
		for (path, bytes) in
			[("photo.png", png(4, 3)), ("cover.png", png(2, 1))]
		{
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
				render_action::fixed_func_route("blog/full-stack-bevy", || {
					(meta(), rsx! {
						<p>"A "<strong>"post"</strong>"."</p>
						<img src="/media/photo.png" alt="A photo"/>
					})
				}),
			]))
			.flush();
		(world, router)
	}

	/// Render the fixture post's document and, given a repo, upload what it
	/// references and write it, all inside the one scope, answering the
	/// document and the blobs uploaded.
	async fn publish(
		world: &mut World,
		router: Entity,
		pds: Option<Pds>,
	) -> Result<(StandardSiteDocument, Vec<BlobRef>)> {
		world
			.run_async_then(async move |world| {
				let publication = harvest();
				let formats = world
					.with(|world: &mut World| {
						world.resource::<StandardSiteContentFormats>().clone()
					})
					.await;
				let path = RelPath::new("blog/full-stack-bevy");
				LivePage::scoped(
					&world.entity(router),
					publication.request(&path),
					async |live| {
						let document = StandardSiteDocument::render(
							&publication,
							harvest_uri(),
							&path,
							&meta(),
							&live,
							&formats,
						)
						.await?;
						let Some(pds) = pds else {
							return (document, Vec::new()).xok();
						};
						let record = AtprotoValue::from_serde(&document)?
							.into_record(&StandardSiteDocument::COLLECTION)?;
						let uploaded =
							live.upload_blob_refs(&pds, &record).await?;
						pds.put(&Rkeyed::new(
							Rkey::from(Tid::from(Timestamp::from_millis(1))),
							document.clone(),
						))
						.await?;
						(document, uploaded).xok()
					},
				)
				.await
			})
			.await
	}

	/// The document carries the post as plain text, as Leaflet blocks with
	/// the image a blob, and its cover a blob, rendered off one tree.
	#[beet_core::test]
	async fn renders_the_body_forms() {
		let (mut world, router) = site().await;
		let (document, _) = publish(&mut world, router, None).await.unwrap();
		document.text_content.xpect_eq(Some("A post.".to_string()));
		document
			.cover_image
			.xpect_eq(Some(BlobRef::of(&png(2, 1), MediaType::Png)));
		let content = document.content.unwrap();
		content.r#type().xpect_eq(LeafletContent::NSID);
		let content = content.into_serde::<LeafletContent>().unwrap();
		match &content.pages[0].blocks[..] {
			[
				BlockSlot {
					block: LeafletBlock::Text(text),
				},
				BlockSlot {
					block: LeafletBlock::Image(image),
				},
			] => {
				text.plaintext.as_str().xpect_eq("A post.");
				image
					.image
					.xpect_eq(BlobRef::of(&png(4, 3), MediaType::Png));
			}
			other => {
				panic!("expected a paragraph and an image, found {other:?}")
			}
		}
	}

	/// Every blob the record references uploads off the tree before the
	/// record is written, and a second run uploads nothing new.
	#[beet_core::test]
	async fn uploads_the_referenced_blobs() {
		let (mut world, router) = site().await;
		let emulator = EmulatorPds::temp();
		let pds = Pds::new(emulator.clone());
		let (_, uploaded) = publish(&mut world, router, Some(pds.clone()))
			.await
			.unwrap();
		uploaded.len().xpect_eq(2);
		let mut stored = emulator.store().list().await.unwrap();
		stored.sort();
		stored.len().xpect_eq(3);
		publish(&mut world, router, Some(pds)).await.unwrap();
		let mut again = emulator.store().list().await.unwrap();
		again.sort();
		again.xpect_eq(stored);
	}

	/// A record referencing a blob no source on its page holds is refused
	/// before anything uploads.
	#[beet_core::test]
	async fn refuses_a_blob_the_page_does_not_hold() {
		let (mut world, router) = site().await;
		let error = world
			.run_async_then(async move |world| {
				LivePage::scoped(
					&world.entity(router),
					Request::get("/blog/full-stack-bevy"),
					async |live| {
						let stranger = BlobRef::of(b"stranger", MediaType::Png);
						let record = AtprotoValue::try_from(value!({
							"image": (Value::from_serde(&stranger).unwrap())
						}))
						.unwrap();
						live.upload_blob_refs(&Pds::temp(), &record).await
					},
				)
				.await
			})
			.await
			.unwrap_err();
		error.to_string().xpect_contains("no source on the page");
	}
}
