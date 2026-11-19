//! What every publish step resolves before it acts.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::exports::bytes::Bytes;
use beet_net::prelude::*;
use beet_ui::prelude::*;

/// The `--dry-run` every step writing to a repo or the index takes.
#[derive(Reflect)]
pub(crate) struct StandardSiteWriteParams {
	/// Report every write a run would make, `created`, `updated` or
	/// `unchanged` as a run logs it, writing and uploading nothing.
	pub dry_run: bool,
}

/// The `--announce` the announce step acts on and the publish step checks
/// before it writes, so an announcement that would be refused refuses the
/// whole run.
#[derive(Reflect)]
pub(crate) struct StandardSiteAnnounceParams {
	/// Announce the published page this slug names, ie `full-stack-bevy`.
	/// Repeatable; a page already announced is left as it is.
	pub announce: Vec<String>,
}

/// One publication as its publish steps see it: the declaration a step's
/// [`PublicationRef`] names, the account's repo, the store its derived files
/// live in, and the listed dated pages beneath it, each a document.
#[derive(Clone)]
pub(super) struct Publishing {
	/// The declaration's entity.
	pub entity: Entity,
	/// The entity owning the url space the pages render in.
	pub router: Entity,
	/// The declaration.
	pub declaration: StandardSitePub,
	/// The repo the publication and its documents live in.
	pub pds: Pds,
	/// The store the declaration resolves by ancestry, where its index lives.
	pub store: BlobStore,
	/// The site identity every url resolves against.
	pub package: PackageConfig,
	/// The publication's url, its natural key.
	pub url: Uri,
	/// The listed dated pages beneath the publication, in route-tree order.
	pub pages: Vec<SyndicationPage>,
	/// The route paths of the drafts beneath the publication, which a run
	/// refuses to name.
	pub drafts: Vec<RelPath>,
	/// Each page renamed beneath the publication, as the document path a
	/// `<Redirect>` sends to and the one it was renamed from.
	renames: Vec<(SmolStr, SmolStr)>,
	/// The world's `<IconSet>`, as the store it resolves and its 512's path.
	icon: Option<(BlobStore, RelPath)>,
	/// The store the world's `<SocialCards>` set is derived in.
	cards: Option<BlobStore>,
}

/// A derived image a record embeds: its bytes and the blob they upload as.
pub(super) struct DerivedImage {
	/// The image.
	pub bytes: Bytes,
	/// What uploading it answers, computed locally.
	pub blob_ref: BlobRef,
}

impl DerivedImage {
	/// The image at `path` in `store`, an error naming the step that derives
	/// it when absent.
	async fn read(store: &BlobStore, path: &RelPath) -> Result<Self> {
		if !store.exists(path).await? {
			bevybail!(
				"`{path}` is not derived yet: the `assets` verb's `<Derive/>` \
				 writes it"
			);
		}
		let media_type = path.media_type().ok_or_else(|| {
			bevyhow!("`{path}` has no extension naming its format")
		})?;
		let bytes = store.get(path).await?;
		Self {
			blob_ref: BlobRef::of(&bytes, media_type),
			bytes,
		}
		.xok()
	}

	/// Upload the image to `pds`, unless `dry_run`.
	pub async fn upload(&self, pds: &Pds, dry_run: bool) -> Result {
		if !dry_run {
			pds.upload_blob(
				self.bytes.clone(),
				self.blob_ref.mime_type.clone(),
			)
			.await?;
		}
		Ok(())
	}
}

impl Publishing {
	/// The publication the publish step `caller` acts on, named by the
	/// [`PublicationRef`] on the step or an ancestor.
	pub async fn resolve(caller: &AsyncEntity) -> Result<Self> {
		caller
			.with_world(|world: &mut World, step: Entity| {
				let entity = world
					.with_state::<AncestorQuery<&PublicationRef>, _>(|refs| {
						refs.get(step).map(|reference| reference.0)
					})
					.map_err(|_| {
						bevyhow!(
							"publish step {step} names no publication: spread \
							 `{{PublicationRef($publication)}}` on it or an \
							 ancestor"
						)
					})?;
				let pds = PdsQuery::resolve(world, entity)?;
				world.with_state::<PublishingQuery, _>(|query| {
					query.publishing(entity, pds)
				})
			})
			.await?
	}

	/// The publication's icon, the `<IconSet>`'s 512, `None` for a world
	/// declaring no set.
	pub async fn icon(&self) -> Result<Option<DerivedImage>> {
		match &self.icon {
			Some((store, path)) => {
				DerivedImage::read(store, path).await?.xmap(Some)
			}
			None => None,
		}
		.xok()
	}

	/// The social card the page at route `page` previews with, the root
	/// route's when the set holds no card of its own, `None` for a world
	/// declaring no set.
	pub async fn card(&self, page: &RelPath) -> Result<Option<DerivedImage>> {
		let Some(store) = &self.cards else {
			return Ok(None);
		};
		let own = SocialCards::card(page);
		let path = match store.exists(&own).await? {
			true => own,
			false => SocialCards::card(&default()),
		};
		DerivedImage::read(store, &path).await?.xmap(Some).xok()
	}

	/// The publication's records as the repo holds them.
	pub async fn records(&self) -> Result<StandardSiteRecords> {
		StandardSiteRecords::read(&self.pds, &self.url).await
	}

	/// The document `path` the page at route `page` publishes at, ie
	/// `/full-stack-bevy`.
	pub fn document_path(&self, page: &SyndicationPage) -> Result<SmolStr> {
		self.declaration.document_path(&page.path).ok_or_else(|| {
			bevyhow!(
				"page '{}' is not beneath the publication at '{}'",
				page.path,
				self.declaration.path
			)
		})
	}

	/// The canonical page of the document at `path`.
	pub fn canonical(&self, path: &str) -> SmolStr {
		format!("{}{path}", self.url).into()
	}

	/// The listed dated page `slug` names, ie `full-stack-bevy` for
	/// `blog/full-stack-bevy`.
	///
	/// # Errors
	/// Errors naming a draft, which is never published, or naming no listed
	/// dated page, listing the slugs that are.
	pub fn page(&self, slug: &str) -> Result<&SyndicationPage> {
		let slug = slug.trim_matches('/');
		let route = self.declaration.path.join(slug);
		if let Some(page) = self.pages.iter().find(|page| page.path == route) {
			return Ok(page);
		}
		if self.drafts.contains(&route) {
			bevybail!("`{slug}` is a draft, and a draft is never published");
		}
		bevybail!(
			"`{slug}` names no listed dated page beneath the publication at \
			 '{}', the pages are: {}",
			self.declaration.path,
			self.pages
				.iter()
				.filter_map(|page| self.document_path(page).ok())
				.map(|path| path.trim_start_matches('/').to_string())
				.collect::<Vec<_>>()
				.join(", ")
		)
	}

	/// The text announcing the listed page `slug` names on `day`, its
	/// description opened by its date line when it is late (see
	/// [`DocMeta::announcement`]).
	///
	/// # Errors
	/// Errors as [`page`](Self::page) does, for a page with no description,
	/// and for a text over [`TextLimit::BSKY_POST_TEXT`], naming the post and
	/// its length.
	pub fn announcement(&self, slug: &str, day: Date) -> Result<String> {
		let page = self.page(slug)?;
		let text = page.meta.announcement(day).ok_or_else(|| {
			bevyhow!(
				"page '{}' has no description, which its announcement's text is",
				page.path
			)
		})?;
		let limit = TextLimit::BSKY_POST_TEXT;
		if let Some(length) = limit.exceeded_by(&text) {
			bevybail!(
				"the announcement of page '{}' is {} graphemes and {} bytes, \
				 over a Bluesky post's limit of {} graphemes and {} bytes: \
				 shorten its description",
				page.path,
				length.graphemes,
				length.bytes,
				limit.graphemes,
				limit.bytes
			);
		}
		Ok(text)
	}

	/// The record the page at route `page` was published as: the document at
	/// its path, else one at a path a `<Redirect>` renamed it from.
	pub fn published<'a>(
		&self,
		records: &'a StandardSiteRecords,
		page: &SyndicationPage,
	) -> Result<Option<&'a RecordEntry<StandardSiteDocument>>> {
		let path = self.document_path(page)?;
		records
			.document(&path)
			.or_else(|| {
				self.renames
					.iter()
					.filter(|(to, _)| *to == path)
					.find_map(|(_, from)| records.document(from))
			})
			.xok()
	}

	/// The contributors the page at route `page` credits, each of its
	/// authors matched to a declared account by display name.
	pub async fn contributors(
		&self,
		world: &AsyncWorld,
		page: &SyndicationPage,
	) -> Result<Vec<Contributor>> {
		let authors = page.meta.authors.clone();
		let path = page.path.clone();
		world
			.with_state::<AtprotoAccountQuery, _>(move |accounts| {
				authors
					.iter()
					.map(|author| {
						accounts
							.by_display_name(author)
							.map(|(_, account)| Contributor::author(account))
							.map_err(|err| {
								bevyhow!(
									"page '{path}' credits {author}: {err}"
								)
							})
					})
					.collect::<Result<Vec<_>>>()
			})
			.await
	}

	/// Write `record` at its rkey unless `dry_run`, logging it `created`,
	/// answering the version written or, dry, the one a write would produce.
	/// For a record no listing must look over, the converge's orphan pass
	/// being for a collection a publication owns.
	pub async fn create<T: AtprotoRecord>(
		pds: &Pds,
		record: Rkeyed<T>,
		dry_run: bool,
	) -> Result<StrongRef> {
		let strong_ref = match dry_run {
			true => StrongRef::new(
				pds.uri(&T::COLLECTION, record.rkey()),
				AtprotoValue::from_serde(&*record)?
					.into_record(&T::COLLECTION)?
					.cid(),
			),
			false => pds.put(&record).await?,
		};
		info!("{:<9} {}", "created", strong_ref.uri);
		strong_ref.xok()
	}

	/// Log each line of `report`, the lines a run prints whether dry or not.
	pub fn log(report: &ConvergeReport) {
		for line in report.to_string().lines() {
			info!("{line}");
		}
	}
}

/// What [`Publishing::resolve`] reads off the world.
#[derive(SystemParam)]
struct PublishingQuery<'w, 's> {
	publications: Query<'w, 's, &'static StandardSitePub>,
	stores: AncestorQuery<'w, 's, &'static BlobStore>,
	syndication: SyndicationQuery<'w, 's>,
	trees: AncestorQuery<'w, 's, &'static RouteTree>,
	routes: Query<'w, 's, (&'static PathPattern, &'static RouteMeta)>,
	icon_sets: Query<'w, 's, (Entity, &'static IconSet)>,
	cards: Query<'w, 's, Entity, With<SocialCards>>,
	redirects: Query<'w, 's, (&'static RedirectTo, &'static PathPattern)>,
}

impl PublishingQuery<'_, '_> {
	/// The publication declared on `entity`, its repo `pds`.
	fn publishing(&self, entity: Entity, pds: Pds) -> Result<Publishing> {
		let declaration =
			self.publications.get(entity).cloned().map_err(|_| {
				bevyhow!(
					"entity {entity} is named as a publication but has no \
					 `StandardSitePub`"
				)
			})?;
		let package = self.syndication.package().clone();
		let pages = self
			.syndication
			.listed(entity, &declaration.path)?
			.into_iter()
			.filter(|page| {
				page.meta.created.is_some()
					&& declaration.document_path(&page.path).is_some()
			})
			.collect();
		let drafts = self
			.routes
			.iter()
			.filter(|(_, meta)| meta.is_draft())
			.map(|(pattern, _)| pattern.annotated_path())
			.filter(|path| declaration.document_path(path).is_some())
			.collect();
		let renames = self
			.redirects
			.iter()
			.filter_map(|(redirect, pattern)| {
				let location = redirect.location(pattern);
				let to = (!location.is_external())
					.then(|| RelPath::from_segments(location.path()))?;
				(
					declaration.document_path(&to)?,
					declaration.document_path(&pattern.annotated_path())?,
				)
					.xmap(Some)
			})
			.collect();
		let icon = self
			.icon_sets
			.iter()
			.next()
			.map(|(icon_entity, icon_set)| {
				self.stores
					.get(icon_entity)
					.map(|store| (store.clone(), icon_set.publication_icon()))
			})
			.transpose()?;
		let cards = self
			.cards
			.iter()
			.next()
			.map(|cards| self.stores.get(cards).cloned())
			.transpose()?;
		Publishing {
			entity,
			renames,
			icon,
			cards,
			router: self.trees.get_entity(entity)?,
			store: self.stores.get(entity)?.clone(),
			url: declaration.url(&package)?,
			declaration,
			pds,
			package,
			pages,
			drafts,
		}
		.xok()
	}
}

/// The fixture site publishing its blog through the publish sequence.
#[cfg(test)]
pub(super) mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::exports::bytes::Bytes;
	use beet_net::prelude::*;
	use beet_ui::prelude::*;

	/// The fixture site with its accounts' repos: `beet` holds the
	/// publication, `pete` authors every post and reposts its announcements.
	pub struct Site {
		pub world: World,
		pub root: Entity,
		pub publication: Entity,
		pub store: BlobStore,
		pub beet: Pds,
		pub pete: Pds,
	}

	/// The social card of the fixture's dated post.
	const CARD: &str = "assets/og/blog/full-stack-bevy.png";

	impl Site {
		/// The fixture site: its blog declared a publication of `beet.org`,
		/// pages wearing a head linking their records, an icon set and a
		/// social card derived into its store.
		pub async fn new() -> Self {
			let beet = EmulatorPds::new(
				Did::parse("did:plc:vgxy56shhs3t3b2mu2avizjf").unwrap(),
				BlobStore::temp(),
			);
			Self::with("https://beet.org", Pds::new(beet)).await
		}

		/// [`Site::new`] at `homepage`, its publication in `publisher`.
		pub async fn with(homepage: &str, publisher: Pds) -> Self {
			let store = BlobStore::temp();
			let icon = IconSet::new("assets/logo/logo.png").publication_icon();
			store.insert(&icon, png(512, 512)).await.unwrap();
			store
				.insert(&RelPath::new(CARD), png(1200, 630))
				.await
				.unwrap();
			let mut world = syndication_world(Some(homepage));
			world
				.get_resource_or_init::<BsxTemplateRegistry>()
				.insert_source(
					"HeadLayout",
					"<html><RouteHead/><body><Slot/></body></html>",
				)
				.unwrap();
			let root = spawn_syndication_router_with(
				&mut world,
				(store.clone(), Layout::new("HeadLayout")),
				rsx! {
					<StandardSitePub path="blog" name="The Full Moon Harvest"/>
					<IconSet src="assets/logo/logo.png"/>
					<SocialCards font="assets/fonts/Inter.ttf"/>
					<Redirect path="blog/full-stack" redirect="/blog/full-stack-bevy"/>
				},
			);
			let beet = world
				.spawn(AtprotoAccount::new(
					"beet.org",
					publisher.did().clone(),
					"Beet",
				))
				.id();
			let pete = world
				.spawn(
					AtprotoAccount::new(
						"pete.beet.org",
						Did::parse("did:plc:hnv7bd4gtxrf7iigjo22qukp").unwrap(),
						"Pete Hayman",
					)
					.with_auto_repost(true),
				)
				.id();
			// the attach lands a network repo first, which the fixture's
			// replace
			world.flush();
			let pete_pds = Pds::new(EmulatorPds::new(
				world.get::<AtprotoAccount>(pete).unwrap().did().clone(),
				BlobStore::temp(),
			));
			world.entity_mut(beet).insert(publisher.clone());
			world.entity_mut(pete).insert(pete_pds.clone());
			let publication = world
				.query_filtered::<Entity, With<StandardSitePub>>()
				.single(&world)
				.unwrap();
			world.entity_mut(publication).insert(AccountRef(beet));
			TemplatePending::settle_owned(&mut world).await;
			Self {
				world,
				root,
				publication,
				store,
				beet: publisher,
				pete: pete_pds,
			}
		}

		/// Run the publish sequence with the command line `args`.
		pub async fn publish(&mut self, args: &str) -> Result {
			let publication = self.publication;
			self.run(
				(PublicationRef(publication), ExchangeSequence, children![
					StandardSitePublish,
					StandardSiteAnnounce,
					StandardSiteIndexWrite
				]),
				args,
			)
			.await
		}

		/// Run `sequence` with the command line `args`.
		pub async fn run(
			&mut self,
			sequence: impl Bundle,
			args: &str,
		) -> Result {
			let request = Request::from_cli_str(&format!("publish {args}"));
			self.world
				.spawn(sequence)
				.run_async_then(async move |entity| {
					entity.call::<Request, Response>(request).await
				})
				.await?
				.into_result()
				.await?;
			Ok(())
		}

		/// The documents the repo holds, in rkey order.
		pub async fn documents(
			&self,
		) -> Vec<RecordEntry<StandardSiteDocument>> {
			self.beet.list().await.unwrap()
		}

		/// The document at `path`.
		pub async fn document(
			&self,
			path: &str,
		) -> RecordEntry<StandardSiteDocument> {
			self.documents()
				.await
				.into_iter()
				.find(|entry| entry.value.path.as_deref() == Some(path))
				.unwrap()
		}

		/// The address and cid of every record the publication's repo holds.
		pub async fn versions(&self) -> Vec<StrongRef> {
			let mut versions = Vec::new();
			for collection in [
				StandardSitePublication::COLLECTION,
				StandardSiteDocument::COLLECTION,
				FeedPost::COLLECTION,
			] {
				for entry in self.beet.list_records(&collection).await.unwrap()
				{
					versions.push(entry.strong_ref());
				}
			}
			versions
		}

		/// The publication's url.
		pub fn publishing_url(&self) -> String {
			self.world
				.get::<StandardSitePub>(self.publication)
				.unwrap()
				.url(self.world.resource::<PackageConfig>())
				.unwrap()
				.to_string()
		}

		/// The posts announcing the publication's documents.
		pub async fn posts(&self) -> Vec<RecordEntry<FeedPost>> {
			let url = self.publishing_url();
			self.beet
				.list::<FeedPost>()
				.await
				.unwrap()
				.into_iter()
				.filter(|entry| {
					entry.value.external().is_some_and(|card| {
						card.external.uri.starts_with(url.as_str())
					})
				})
				.collect()
		}

		/// Delete every record the publication wrote, so a live account is
		/// left as it was found.
		pub async fn clean(&self) {
			let url = Uri::parse(&self.publishing_url()).unwrap();
			for post in self.posts().await {
				self.beet
					.delete_record(&FeedPost::COLLECTION, post.rkey())
					.await
					.unwrap();
			}
			let records =
				StandardSiteRecords::read(&self.beet, &url).await.unwrap();
			for entry in records.documents {
				self.beet
					.delete_record(
						&StandardSiteDocument::COLLECTION,
						entry.rkey(),
					)
					.await
					.unwrap();
			}
			if let Some(entry) = records.publication {
				self.beet
					.delete_record(
						&StandardSitePublication::COLLECTION,
						entry.rkey(),
					)
					.await
					.unwrap();
			}
		}

		/// The committed index's bytes, `None` before the first publish.
		pub async fn index(&self) -> Option<Bytes> {
			let path = RelPath::new(StandardSitePub::INDEX);
			match self.store.exists(&path).await.unwrap() {
				true => Some(self.store.get(&path).await.unwrap()),
				false => None,
			}
		}
	}

	/// A run writes the publication and the one page it names, its `content`
	/// the post as a scene, and its index; a second writes nothing and leaves
	/// the index byte for byte.
	#[beet_core::test]
	async fn publishes_once_then_holds() {
		let mut site = Site::new().await;
		site.publish("--only=full-stack-bevy").await.unwrap();
		let publication = site
			.beet
			.list::<StandardSitePublication>()
			.await
			.unwrap()
			.pop()
			.unwrap();
		publication
			.value
			.icon
			.xpect_eq(Some(BlobRef::of(&png(512, 512), MediaType::Png)));
		let documents = site.documents().await;
		documents.len().xpect_eq(1);
		let document = &documents[0].value;
		document.path.as_deref().xpect_eq(Some("/full-stack-bevy"));
		document
			.site
			.at_uri()
			.xpect_eq(Some(publication.uri.clone()));
		document.contributors[0]
			.did
			.as_str()
			.xpect_eq("did:plc:hnv7bd4gtxrf7iigjo22qukp");
		// the repo answers the post as a scene body, read with no registry
		let content = document.content.clone().unwrap();
		content.r#type().clone().xpect_eq(SceneBody::NSID);
		SceneBody::from_value(Value::Map(content.fields().clone()))
			.unwrap()
			.entities
			.is_empty()
			.xpect_false();
		let versions = site.versions().await;
		let index = site.index().await.unwrap();
		str::from_utf8(&index)
			.unwrap()
			.xpect_contains("/full-stack-bevy");
		site.publish("--only=full-stack-bevy").await.unwrap();
		site.versions().await.xpect_eq(versions);
		site.index().await.unwrap().xpect_eq(index);
	}

	/// A publication naming `pub.leaflet.content` publishes each document's
	/// `content` as Leaflet blocks, its image blobs uploaded, and a second run
	/// writes nothing.
	#[cfg(feature = "standard_site_extra")]
	#[beet_core::test]
	async fn publishes_leaflet_content_once_then_holds() {
		let mut site = Site::new().await;
		site.world
			.get_mut::<StandardSitePub>(site.publication)
			.unwrap()
			.content = Some(LeafletContent::NSID);
		site.publish("--only=full-stack-bevy").await.unwrap();
		site.document("/full-stack-bevy")
			.await
			.value
			.content
			.unwrap()
			.r#type()
			.xpect_eq(LeafletContent::NSID);
		let versions = site.versions().await;
		site.publish("--only=full-stack-bevy").await.unwrap();
		site.versions().await.xpect_eq(versions);
	}

	/// A bare run updates what is published and creates nothing.
	#[beet_core::test]
	async fn a_bare_run_creates_nothing() {
		let mut site = Site::new().await;
		site.publish("--only=full-stack-bevy").await.unwrap();
		let published = site.document("/full-stack-bevy").await;
		site.beet
			.put(&Rkeyed::new(
				published.rkey().clone(),
				StandardSiteDocument {
					title: "Stale".into(),
					..published.value.clone()
				},
			))
			.await
			.unwrap();
		site.publish("").await.unwrap();
		let documents = site.documents().await;
		documents.len().xpect_eq(1);
		documents[0]
			.value
			.title
			.as_str()
			.xpect_eq("Full Stack Bevy");
		documents[0].uri.clone().xpect_eq(published.uri);
	}

	/// A page renamed behind a `<Redirect>` updates the record it was
	/// published as, rather than orphaning it beside a new one.
	#[beet_core::test]
	async fn follows_a_rename() {
		let mut site = Site::new().await;
		site.publish("--only=full-stack-bevy").await.unwrap();
		let published = site.document("/full-stack-bevy").await;
		// published before the rename, at its old path
		site.beet
			.put(&Rkeyed::new(
				published.rkey().clone(),
				StandardSiteDocument {
					path: Some("/full-stack".into()),
					..published.value
				},
			))
			.await
			.unwrap();
		site.publish("").await.unwrap();
		let documents = site.documents().await;
		documents.len().xpect_eq(1);
		documents[0].uri.clone().xpect_eq(published.uri);
		documents[0]
			.value
			.path
			.as_deref()
			.xpect_eq(Some("/full-stack-bevy"));
	}

	/// A draft is never published, named or not, and a slug naming no page
	/// lists the ones it could.
	#[beet_core::test]
	async fn refuses_a_draft() {
		let mut site = Site::new().await;
		site.publish("--only=wip")
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("`wip` is a draft");
		site.publish("--only=nope")
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("ecs-router, full-stack-bevy");
		site.publish("--all").await.unwrap();
		site.documents()
			.await
			.into_iter()
			.map(|entry| entry.value.path.unwrap())
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				SmolStr::new("/ecs-router"),
				SmolStr::new("/full-stack-bevy"),
			]);
	}

	/// A named draft refuses before the repo is read, so the refusal holds
	/// whether or not the repo answers.
	#[beet_core::test]
	async fn refuses_before_reading_the_repo() {
		let unreachable = XrpcPds::new(
			Did::parse("did:plc:vgxy56shhs3t3b2mu2avizjf").unwrap(),
		)
		.with_endpoint("http://127.0.0.1:9");
		let mut site =
			Site::with("https://beet.org", Pds::new(unreachable)).await;
		site.publish("--only=wip --dry-run")
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("`wip` is a draft");
	}

	/// `--prune` deletes a document whose page is gone, never one merely
	/// outside `--only`.
	#[beet_core::test]
	async fn prunes_only_the_gone() {
		let mut site = Site::new().await;
		site.publish("--all").await.unwrap();
		let published = site.document("/ecs-router").await;
		site.beet
			.put(&Rkeyed::new(Tid::now().into(), StandardSiteDocument {
				path: Some("/gone".into()),
				..published.value
			}))
			.await
			.unwrap();
		site.publish("--prune --only=full-stack-bevy")
			.await
			.unwrap();
		let mut paths = site
			.documents()
			.await
			.into_iter()
			.map(|entry| entry.value.path.unwrap())
			.collect::<Vec<_>>();
		paths.sort();
		paths.xpect_eq(vec![
			SmolStr::new("/ecs-router"),
			SmolStr::new("/full-stack-bevy"),
		]);
	}

	/// An announcement links the page's card and both records, the document
	/// points back at it, its author reposts it; a run interrupted after the
	/// post resumes without posting twice.
	#[beet_core::test]
	async fn announces_once_and_resumes() {
		announce_suite(&mut Site::new().await).await;
	}

	/// The announcement suite, run against any publishing repo: the emulator
	/// here, the throwaway account behind the live gate.
	async fn announce_suite(site: &mut Site) {
		let canonical = format!(
			"{}/full-stack-bevy",
			site.publishing_url().trim_end_matches('/')
		);
		site.publish("--only=full-stack-bevy --announce=full-stack-bevy")
			.await
			.unwrap();
		let posts = site.posts().await;
		posts.len().xpect_eq(1);
		let card = posts[0].value.external().unwrap().external;
		card.uri.as_str().xpect_eq(canonical.as_str());
		card.thumb
			.xpect_eq(Some(BlobRef::of(&png(1200, 630), MediaType::Png)));
		card.associated_refs.len().xpect_eq(2);
		// the fixture post is long past its week, so it opens dated
		posts[0]
			.value
			.text
			.as_str()
			.xpect_eq("Published 11 July 2025\n\nall about Full Stack Bevy");
		let post = posts[0].strong_ref();
		let document = site.document("/full-stack-bevy").await;
		document
			.value
			.bsky_post_ref
			.clone()
			.xpect_eq(Some(post.clone()));
		let reposts = site.pete.list::<FeedRepost>().await.unwrap();
		reposts.len().xpect_eq(1);
		reposts[0].value.subject.clone().xpect_eq(post.clone());
		str::from_utf8(&site.index().await.unwrap())
			.unwrap()
			.xpect_contains(&post.uri.to_string());
		// a second run writes nothing, the index byte for byte
		let (versions, index) = (site.versions().await, site.index().await);
		site.publish("--only=full-stack-bevy --announce=full-stack-bevy")
			.await
			.unwrap();
		site.versions().await.xpect_eq(versions);
		site.index().await.xpect_eq(index);
		// interrupted after the post, before the document took it up
		site.beet
			.put(&Rkeyed::new(
				document.rkey().clone(),
				StandardSiteDocument {
					bsky_post_ref: None,
					..document.value
				},
			))
			.await
			.unwrap();
		site.publish("--announce=full-stack-bevy").await.unwrap();
		site.posts().await.len().xpect_eq(1);
		site.document("/full-stack-bevy")
			.await
			.value
			.bsky_post_ref
			.xpect_eq(Some(post));
		site.pete
			.list::<FeedRepost>()
			.await
			.unwrap()
			.len()
			.xpect_eq(1);
	}

	/// The publish sequence against the throwaway account, everything it
	/// wrote deleted before and after.
	#[cfg(not(target_arch = "wasm32"))]
	#[ignore = "live: writes to the throwaway account"]
	#[beet_core::test(timeout_ms = 180_000)]
	async fn publishes_to_a_live_repo() {
		let Some((_, publisher)) = XrpcPds::throwaway() else {
			return;
		};
		let mut site = Site::with("https://example.com", publisher).await;
		site.clean().await;
		announce_suite(&mut site).await;
		site.clean().await;
	}

	/// An announcement over a Bluesky post's limit, only once its date line
	/// opens it, refuses the run before anything is written, the error
	/// naming the post and its length.
	#[beet_core::test]
	async fn refuses_an_announcement_over_the_limit() {
		let mut site = Site::new().await;
		// 24 graphemes of date line beside 277 of description
		for mut meta in site
			.world
			.query::<&mut DocMeta>()
			.iter_mut(&mut site.world)
			.filter(|meta| meta.title.as_deref() == Some("Full Stack Bevy"))
		{
			meta.description = Some("a".repeat(277));
		}
		for args in ["", " --dry-run"] {
			site.publish(&format!(
				"--only=full-stack-bevy --announce=full-stack-bevy{args}"
			))
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("page 'blog/full-stack-bevy' is 301 graphemes")
			.xpect_contains("limit of 300 graphemes");
		}
		site.versions().await.xpect_empty();
		site.index().await.xpect_none();
	}

	/// A dry run writes nothing anywhere.
	#[beet_core::test]
	async fn a_dry_run_writes_nothing() {
		let mut site = Site::new().await;
		site.publish("--all --announce=full-stack-bevy --dry-run")
			.await
			.unwrap();
		site.versions().await.xpect_empty();
		site.pete.list::<FeedRepost>().await.unwrap().xpect_empty();
		site.index().await.xpect_none();
	}
}
