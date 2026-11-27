//! The standard site index: where each record of a publication went, the
//! derived file the served site reads instead of the public repo.
//!
//! # What it serves
//!
//! A [`StandardSitePub`] reads its index from the store it resolves by
//! ancestry once it is declared, the build's `Ready` waiting on the read, and
//! lands it on its own entity. From then on:
//!
//! - the well-known route, `/.well-known/site.standard.publication/<path>`
//!   (the bare well-known for a publication at the site root), answers the
//!   publication's uri as `text/plain`, so a reader can verify the site
//!   claims the record;
//! - `RouteHead` links the publication from every page at or under its path,
//!   and a page the index knows to its document, the `<link>`s a reader
//!   verifies a document by.
//!
//! No index, no well-known route and no links: a publication not yet
//! published claims nothing. A stale one still serves, with a warning naming
//! `publish`; one that does not read serves nothing, the error reported
//! rather than fatal.
//!
//! # Its provenance
//!
//! The index is a copy of the public repo, rebuilt by listing it, so its
//! [`Provenance`] names no sources and no `ttl`. Naming each record as a
//! source would cost a network read per document on every freshness check,
//! `beet check` and every boot included, and fail offline; a `ttl` would call
//! an unchanged index stale by the clock. The comparison against the public repo is
//! the probe's, run on purpose.
//!
//! # Who writes it
//!
//! `publish` alone ([`Derivation::written_by`]): `<Derive/>` never
//! regenerates it, so a deploy serves the index exactly as committed. A
//! regeneration inside the assembled copy would diverge from git, the index
//! being the one place the rkeys are reviewed, and would make every deploy
//! depend on the public repo answering a listing.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Where each record of a publication lives: the publication itself and each
/// standard site document by its `path`, each with the cid it was written at,
/// a document with the announcement its comments hang off.
///
/// The body of the derived file a [`StandardSitePub`] declares at its
/// `index`, beside the file's [`Provenance`], and the component the
/// declaration carries once the file is read. The converge reads it for the
/// rkey each record was minted at before it writes, so it is the one memory
/// of them, rebuilt only by listing the public repo
/// ([`from_repo`](Self::from_repo)).
#[derive(
	Debug, Clone, PartialEq, Eq, Component, Reflect, Serialize, Deserialize,
)]
#[reflect(Component, Serialize, Deserialize)]
pub struct StandardSiteIndex {
	/// The publication record.
	pub publication: StrongRef,
	/// Each document of the publication, ordered by path.
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub documents: Vec<StandardSiteIndexDocument>,
}

/// One standard site document in a [`StandardSiteIndex`].
#[derive(Debug, Clone, PartialEq, Eq, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
pub struct StandardSiteIndexDocument {
	/// The document's natural key, its `path`, ie `/full-stack-bevy`.
	pub path: SmolStr,
	/// The document record.
	pub uri: AtUri,
	/// The document's cid at the version written.
	pub cid: Cid,
	/// The Bluesky post announcing the document, once announced.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub bsky_post: Option<StrongRef>,
}

impl StandardSiteIndex {
	/// The index of the publication at `url` as `pds` holds it: the
	/// publication found by its natural key, and every document naming it
	/// with a `path`.
	///
	/// # Errors
	/// Errors when the public repo holds no publication at `url`, or a record of
	/// either collection does not read as its lexicon.
	pub async fn from_repo(pds: &Pds, url: &Uri) -> Result<Self> {
		StandardSiteRecords::read(pds, url)
			.await?
			.index()
			.ok_or_else(|| {
				bevyhow!(
					"{} holds no publication at `{url}`: `{}` writes it",
					pds.describe(),
					StandardSitePub::WRITTEN_BY
				)
			})
	}

	/// The document at `path`, the document's own `path`, ie
	/// `/full-stack-bevy`.
	pub fn document(&self, path: &str) -> Option<&StandardSiteIndexDocument> {
		self.documents.iter().find(|document| document.path == path)
	}

	/// Read the index `publication` declares from the store it resolves by
	/// ancestry and land it, or nothing when absent, warning when stale.
	pub(super) async fn load(publication: &AsyncEntity) -> Result {
		let (store, derivation) = publication
			.with_state::<(AncestorQuery<&BlobStore>, Query<&Derivation>), _>(
				|entity, (stores, derivations)| -> Result<_> {
					(
						stores.get(entity)?.clone(),
						derivations.get(entity)?.clone(),
					)
						.xok()
				},
			)
			.await??;
		let index = match derivation.read(&store).await? {
			Some(body) => body.get_typed::<Self>()?.xmap(Some),
			None => {
				debug!(
					"no standard site index at `{}`: nothing published yet",
					derivation.path
				);
				None
			}
		};
		if index.is_some() {
			let freshness = Derivation::fresh(
				publication.world(),
				publication.id(),
				&store,
			)
			.await?;
			if !freshness.is_fresh() {
				warn!(
					"standard site index `{}` is stale: {freshness}, `{}` \
					 refreshes it",
					derivation.path,
					StandardSitePub::WRITTEN_BY
				);
			}
		}
		let entity = publication.id();
		publication
			.world()
			.with(move |world: &mut World| Self::land(world, entity, index))
			.await
	}

	/// Land `index` on `publication`, replacing any earlier one and its
	/// well-known route, and serve the well-known route while there is one.
	fn land(
		world: &mut World,
		publication: Entity,
		index: Option<Self>,
	) -> Result {
		let retired = world
			.with_state::<(Query<&Children>, Query<(), With<WellKnownRoute>>), _>(
				|(children, routes)| {
					children
						.iter_descendants(publication)
						.filter(|child| routes.contains(*child))
						.collect::<Vec<_>>()
				},
			);
		for route in retired {
			world.entity_mut(route).despawn();
		}
		let Some(index) = index else {
			world.entity_mut(publication).remove::<Self>();
			return Ok(());
		};
		let path = world
			.get::<StandardSitePub>(publication)
			.ok_or_else(|| {
				bevyhow!("entity {publication} has no `StandardSitePub`")
			})?
			.well_known_path();
		let uri = SmolStr::new(index.publication.uri.to_string());
		world.entity_mut(publication).insert(index);
		world.spawn((
			ChildOf(publication),
			WellKnownRoute,
			route::exchange(
				path.as_str(),
				Action::<Request, Response>::new_async(
					async move |_: ActionContext<Request>| -> Result<Response> {
						Response::ok_body(uri.to_string(), MediaType::Text)
							.xok()
					},
				),
			),
			HttpMethod::Get,
			ExportStrategy::Static,
		));
		world.flush();
		Ok(())
	}
}

/// A publication's records as its public repo holds them, read by listing both
/// collections: the memory the publish steps converge against and the index
/// is rebuilt from. Every record carries its natural key, so the listing finds
/// a record whatever an index lost, a run interrupted before the index was
/// written included.
#[derive(Debug, Default, Clone)]
pub struct StandardSiteRecords {
	/// The publication, found by its natural key, its `url`; `None` before
	/// the first publish.
	pub publication: Option<RecordEntry<StandardSitePublication>>,
	/// Every document naming the publication, in rkey order.
	pub documents: Vec<RecordEntry<StandardSiteDocument>>,
}

impl StandardSiteRecords {
	/// The records of the publication at `url` in `pds`. Where several
	/// publications share the url the earliest wins, and the converge reports
	/// the rest as orphans.
	///
	/// # Errors
	/// Errors when a record of either collection does not read as its
	/// lexicon.
	pub async fn read(pds: &Pds, url: &Uri) -> Result<Self> {
		let Some(publication) = pds
			.list::<StandardSitePublication>()
			.await?
			.into_iter()
			.find(|entry| &entry.value.url == url)
		else {
			return Self::default().xok();
		};
		let documents = pds
			.list::<StandardSiteDocument>()
			.await?
			.into_iter()
			.filter(|entry| {
				entry.value.site.at_uri().as_ref() == Some(&publication.uri)
			})
			.collect();
		Self {
			publication: Some(publication),
			documents,
		}
		.xok()
	}

	/// The document whose `path` is `path`, ie `/full-stack-bevy`.
	pub fn document(
		&self,
		path: &str,
	) -> Option<&RecordEntry<StandardSiteDocument>> {
		self.documents
			.iter()
			.find(|entry| entry.value.path.as_deref() == Some(path))
	}

	/// The index of these records, `None` without a publication: every
	/// document with a `path`, ordered by it.
	pub fn index(&self) -> Option<StandardSiteIndex> {
		let publication = self.publication.as_ref()?;
		let mut documents = self
			.documents
			.iter()
			.filter_map(|entry| {
				StandardSiteIndexDocument {
					path: entry.value.path.clone()?,
					uri: entry.uri.clone(),
					cid: entry.cid.clone(),
					bsky_post: entry.value.bsky_post_ref.clone(),
				}
				.xmap(Some)
			})
			.collect::<Vec<_>>();
		documents.sort_by(|left, right| left.path.cmp(&right.path));
		StandardSiteIndex {
			publication: publication.strong_ref(),
			documents,
		}
		.xmap(Some)
	}
}

/// Marks the well-known route a [`StandardSitePub`] serves while it has an
/// index, so a reload replaces it.
#[derive(Component)]
struct WellKnownRoute;

#[cfg(test)]
mod test {
	use super::super::test_fixtures::*;
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_ui::prelude::*;

	/// The fixture site wearing a layout whose head is `RouteHead`, its public repo
	/// store holding `index` when given, the fixture publication declared at
	/// its root and its index loaded.
	async fn served(index: Option<StandardSiteIndex>) -> (World, Entity) {
		let store = BlobStore::temp();
		if let Some(index) = index {
			write_index(&store, &index).await;
		}
		let mut world = syndication_world(Some("https://beet.org"));
		store
			.insert(
				&fwd_path!("HeadLayout.bsx"),
				"<html><RouteHead/><body><slot/></body></html>",
			)
			.await
			.unwrap();
		let root = spawn_syndication_router_with(
			&mut world,
			(store, Layout::document(fwd_path!("HeadLayout.bsx"))),
			rsx! { <StandardSitePub path={fwd_path!("blog")} name="The Full Moon Harvest"/> },
		);
		TemplatePending::settle_owned(&mut world).await;
		(world, root)
	}

	/// `path`'s response, its status and its body.
	async fn get(
		world: &mut World,
		root: Entity,
		path: &str,
	) -> (StatusCode, String) {
		let response =
			world.entity_mut(root).exchange(Request::get(path)).await;
		let status = response.status();
		(status, response.text().await.unwrap())
	}

	const WELL_KNOWN: &str = ".well-known/site.standard.publication/blog";
	const PUBLICATION_LINK: &str = r#"rel="site.standard.publication""#;
	const DOCUMENT_LINK: &str = r#"rel="site.standard.document""#;

	/// The index serves the well-known body, the publication's link on every
	/// page beneath it and the document's on a page it knows.
	#[beet_core::test]
	async fn serves_the_well_known_and_the_links() {
		let index = harvest_index();
		let (mut world, root) = served(Some(index.clone())).await;
		let (status, body) = get(&mut world, root, WELL_KNOWN).await;
		status.xpect_eq(StatusCode::OK);
		body.xpect_eq(harvest_uri().to_string());
		let publication =
			format!(r#"{PUBLICATION_LINK} href="{}""#, harvest_uri());
		let document =
			format!(r#"{DOCUMENT_LINK} href="{}""#, index.documents[0].uri);
		get(&mut world, root, "blog/full-stack-bevy")
			.await
			.1
			.xpect_contains(&publication)
			.xpect_contains(&document);
		get(&mut world, root, "blog/ecs-router")
			.await
			.1
			.xpect_contains(&publication)
			.xnot()
			.xpect_contains(DOCUMENT_LINK);
		get(&mut world, root, "")
			.await
			.1
			.xnot()
			.xpect_contains(PUBLICATION_LINK);
	}

	/// Without an index the site claims nothing: no well-known route and no
	/// links.
	#[beet_core::test]
	async fn claims_nothing_without_an_index() {
		let (mut world, root) = served(None).await;
		get(&mut world, root, WELL_KNOWN)
			.await
			.0
			.xpect_eq(StatusCode::NOT_FOUND);
		get(&mut world, root, "blog/full-stack-bevy")
			.await
			.1
			.xnot()
			.xpect_contains(PUBLICATION_LINK)
			.xnot()
			.xpect_contains(DOCUMENT_LINK);
	}

	/// The public repo holding the fixture publication, a document of it, a document
	/// of another publication and a loose document, at the address each was
	/// written.
	async fn public_repo() -> (Pds, StandardSitePublication) {
		let pds = Pds::temp();
		let publication = StandardSitePublication::from_declaration(
			&harvest(),
			&PackageConfig {
				homepage: Some(Url::coerce("https://beet.org")),
				..default()
			},
			&ThemeTones::default(),
			None,
		)
		.unwrap();
		let rkey =
			|millis: i64| Rkey::from(Tid::from(Timestamp::from_millis(millis)));
		let site = pds
			.put(&Rkeyed::new(rkey(1), publication.clone()))
			.await
			.unwrap()
			.uri;
		let elsewhere = AtUri::parse(
			"at://did:plc:vgxy56shhs3t3b2mu2avizjf/site.standard.publication/3mw72aaeuj22z",
		)
		.unwrap();
		let document = |site: &AtUri, title: &str, path: &str| {
			StandardSiteDocument::from_page(
				&harvest(),
				site.clone(),
				&FwdPath::new(format!("blog/{path}")).unwrap(),
				&post(title, "2025-07-11"),
			)
			.unwrap()
		};
		for (millis, record) in [
			(2, document(&site, "Full Stack Bevy", "full-stack-bevy")),
			(3, document(&site, "ECS Router", "ecs-router")),
			(4, document(&elsewhere, "Elsewhere", "elsewhere")),
			(5, StandardSiteDocument {
				path: None,
				..document(&site, "Loose", "loose")
			}),
		] {
			pds.put(&Rkeyed::new(rkey(millis), record)).await.unwrap();
		}
		(pds, publication)
	}

	/// A rebuild lists the publication and its documents with a path, ordered
	/// by path, and nothing of another publication.
	#[beet_core::test]
	async fn rebuilds_from_the_repo() {
		let (pds, publication) = public_repo().await;
		let index = StandardSiteIndex::from_repo(&pds, &publication.url)
			.await
			.unwrap();
		index
			.publication
			.uri
			.collection()
			.xpect_eq(StandardSitePublication::COLLECTION);
		index
			.documents
			.iter()
			.map(|document| document.path.as_str())
			.collect::<Vec<_>>()
			.xpect_eq(vec!["/ecs-router", "/full-stack-bevy"]);
		StandardSiteIndex::from_repo(
			&pds,
			&Uri::parse("https://beet.org/docs").unwrap(),
		)
		.await
		.unwrap_err()
		.to_string()
		.xpect_contains("`publish` writes it");
	}

	/// The declaration is its index's producer: ensuring it lists the account
	/// it resolves and writes the index with its provenance.
	#[beet_core::test]
	async fn derives_its_index() {
		let (pds, publication) = public_repo().await;
		let store = BlobStore::temp();
		let mut world = syndication_world(Some("https://beet.org"));
		let entity =
			world.spawn((harvest(), pds.clone(), store.clone())).flush();
		let ensured = store.clone();
		world
			.run_async_then(async move |world| {
				Derivation::ensure(&world, entity, &ensured).await
			})
			.await
			.unwrap()
			.xpect_eq(Freshness::Missing);
		harvest()
			.derivation()
			.read(&store)
			.await
			.unwrap()
			.unwrap()
			.get_typed::<StandardSiteIndex>()
			.unwrap()
			.xpect_eq(
				StandardSiteIndex::from_repo(&pds, &publication.url)
					.await
					.unwrap(),
			);
	}
}
