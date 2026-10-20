//! [`Derivation`]: a derived file declared, checked and regenerated.
//!
//! The pattern, its words and where each kind of derived file is stored are
//! [`Provenance`]'s module docs; this is the store side.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Declares the derived file at `path`, in the [`BlobStore`] this entity
/// resolves by ancestry, which this entity's own action produces.
///
/// The producing action is an `Action<BlobStore, EntityBody>`. Handed the
/// store the file is ensured in, it reads what it needs (its file sources from
/// that same store, so a derivation ensured into a deploy's stage reads the
/// staged copy) and answers the document: an [`EntityBody`] carrying a
/// [`Provenance`] that names each source with the digest it read.
/// [`Derivation::ensure`] stamps the time and the declared `ttl` onto that
/// provenance and writes the body in the format `path`'s extension names,
/// pretty where the format has one, so a producer says what it read and the
/// declaration how long that holds.
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_action::prelude::*;
/// # use beet_net::prelude::*;
/// # async fn run() -> Result {
/// let mut world = AsyncPlugin.into_world();
/// let store = BlobStore::temp();
/// store.insert(&RelPath::new("notes.md"), "# Notes").await?;
/// let entity = world
/// 	.spawn((
/// 		Derivation::new("derived/notes.json"),
/// 		Action::<BlobStore, EntityBody>::new_async(
/// 			async |cx: ActionContext<BlobStore>| {
/// 				let notes = cx.input.get(&RelPath::new("notes.md")).await?;
/// 				EntityBody::default().with(&Provenance::new([
/// 					ProvenanceSource::file("notes.md", &notes),
/// 				]))
/// 			},
/// 		),
/// 	))
/// 	.id();
/// // written the first time, then again only once `notes.md` changes
/// world
/// 	.run_async_then(async move |world| {
/// 		Derivation::ensure(&world, entity, &store).await
/// 	})
/// 	.await?
/// 	.xpect_eq(Freshness::Missing);
/// # Ok(()) }
/// ```
///
/// A derivation is declared by whatever owns its file, beside that file's
/// producing action, and ensured by a step gathering every one a document
/// declares, so nothing regenerates until something asks.
///
/// A record source is read from the repo of the `Pds` in the world holding its
/// did. A record no repo here holds cannot be seen from here and counts as
/// unchanged, so a derivation reading one declares a `ttl`.
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct Derivation {
	/// The derived file's path in its store, its extension naming the format.
	pub path: RelPath,
	/// How long a regeneration holds for an input nothing local can hash,
	/// stamped onto the [`Provenance`] it writes.
	pub ttl: Option<Duration>,
}

impl Derivation {
	/// The derived file at `path`, holding until a source changes.
	pub fn new(path: impl Into<RelPath>) -> Self {
		Self {
			path: path.into(),
			ttl: None,
		}
	}

	/// This derivation expiring `ttl` after each regeneration.
	pub fn with_ttl(mut self, ttl: Duration) -> Self {
		self.ttl = Some(ttl);
		self
	}

	/// Every entity declaring a derivation in `root`'s subtree, `root`
	/// included: what a document declares.
	pub fn declared(world: &mut World, root: Entity) -> Vec<Entity> {
		world.with_state::<(Query<&Children>, Query<(), With<Derivation>>), _>(
			|(children, derivations)| {
				children
					.iter_descendants_inclusive(root)
					.filter(|entity| derivations.contains(*entity))
					.collect()
			},
		)
	}

	/// The derived file in `store`, `None` when absent.
	pub async fn read(&self, store: &BlobStore) -> Result<Option<EntityBody>> {
		match store.exists(&self.path).await? {
			true => self
				.media_type()?
				.deserialize::<EntityBody>(&store.get(&self.path).await?)?
				.xmap(Some)
				.xok(),
			false => None.xok(),
		}
	}

	/// Whether the derived file `entity` declares is fresh in `store`, and
	/// why not.
	pub async fn fresh(
		world: &AsyncWorld,
		entity: Entity,
		store: &BlobStore,
	) -> Result<Freshness> {
		Self::check(world, entity, store)
			.await
			.map(|(_, freshness)| freshness)
	}

	/// Regenerate the derived file `entity` declares in `store` when it is
	/// stale or missing, and do nothing otherwise, answering what was found.
	pub async fn ensure(
		world: &AsyncWorld,
		entity: Entity,
		store: &BlobStore,
	) -> Result<Freshness> {
		let (derivation, freshness) = Self::check(world, entity, store).await?;
		if freshness.is_fresh() {
			debug!("derived file `{}` is fresh", derivation.path);
		} else {
			derivation.regenerate(world.entity(entity), store).await?;
			info!(
				"derived file `{}` regenerated: {freshness}",
				derivation.path
			);
		}
		freshness.xok()
	}

	/// The derivation `entity` declares, and its freshness in `store`.
	async fn check(
		world: &AsyncWorld,
		entity: Entity,
		store: &BlobStore,
	) -> Result<(Self, Freshness)> {
		let derivation = world.entity(entity).get_cloned::<Self>().await?;
		let digests = SourceDigests::new(world, store.clone()).await;
		let freshness = derivation.freshness(store, &digests).await?;
		(derivation, freshness).xok()
	}

	/// The derived file's format, named by its extension.
	fn media_type(&self) -> Result<MediaType> {
		self.path.media_type().ok_or_else(|| {
			bevyhow!(
				"derived file `{}` has no extension naming its format",
				self.path
			)
		})
	}

	/// The module docs' freshness of the file in `store`.
	async fn freshness(
		&self,
		store: &BlobStore,
		digests: &SourceDigests,
	) -> Result<Freshness> {
		if !store.exists(&self.path).await? {
			return Freshness::Missing.xok();
		}
		let bytes = store.get(&self.path).await?;
		// a file that is no derived document is regenerated, never trusted
		let provenance = match self
			.media_type()?
			.deserialize::<EntityBody>(&bytes)
			.and_then(|body| body.get::<Provenance>())
		{
			Ok(provenance) => provenance,
			Err(error) => return Freshness::Invalid(error.to_string()).xok(),
		};
		if provenance.expired(Timestamp::now()) {
			return Freshness::Expired.xok();
		}
		match digests.first_changed(&provenance.sources).await? {
			Some(source) => Freshness::Changed(source),
			None => Freshness::Fresh,
		}
		.xok()
	}

	/// Run the producing action and write the document it answers, stamped
	/// with the time and this declaration's ttl.
	async fn regenerate(
		&self,
		entity: AsyncEntity,
		store: &BlobStore,
	) -> Result {
		let produces = entity
			.get::<ActionMeta, _>(|meta| {
				meta.matches::<BlobStore, EntityBody>()
			})
			.await
			.unwrap_or(false);
		if !produces {
			bevybail!(
				"derived file `{}` has no producing action: entity {} needs an \
				 `Action<BlobStore, EntityBody>` beside its `Derivation`",
				self.path,
				entity.id()
			);
		}
		let mut body =
			entity.call::<BlobStore, EntityBody>(store.clone()).await?;
		if !body.contains::<Provenance>() {
			bevybail!(
				"the producing action of derived file `{}` answered no \
				 `Provenance`: a derived file names the sources it was computed from",
				self.path
			);
		}
		body.insert(&Provenance {
			created: Timestamp::now(),
			ttl: self.ttl,
			..body.get::<Provenance>()?
		})?;
		let bytes = self
			.media_type()?
			.serialize_with_options(&body, SerializeOptions { pretty: true })?;
		store.insert(&self.path, bytes).await
	}
}

/// What [`Derivation::fresh`] found: fresh, or why a derived file is stale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Freshness {
	/// Every source's digest unchanged and the ttl unexpired.
	Fresh,
	/// No file at the declared path.
	Missing,
	/// The file is no entity body carrying a [`Provenance`], ie written by hand.
	Invalid(String),
	/// The ttl ran out.
	Expired,
	/// A source's digest changed, or the source is gone.
	Changed(ProvenanceSource),
}

impl Freshness {
	/// Whether the file needs no regeneration.
	pub fn is_fresh(&self) -> bool { *self == Self::Fresh }
}

impl core::fmt::Display for Freshness {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::Fresh => write!(f, "fresh"),
			Self::Missing => write!(f, "missing"),
			Self::Invalid(reason) => {
				write!(f, "not a derived document: {reason}")
			}
			Self::Expired => write!(f, "past its ttl"),
			Self::Changed(source) => write!(f, "{source} changed"),
		}
	}
}

/// What each source of a [`Provenance`] reads as now: a file from the store
/// the derived file is in, a record from the repo holding its did.
struct SourceDigests {
	store: BlobStore,
	/// Every repo in the world, matched to a record source by did.
	#[cfg(feature = "dag_cbor")]
	repos: Vec<Pds>,
}

impl SourceDigests {
	/// The digests a derived file in `store` is checked against.
	async fn new(world: &AsyncWorld, store: BlobStore) -> Self {
		// without the data model no repo holds a record
		#[cfg(not(feature = "dag_cbor"))]
		let _ = world;
		Self {
			store,
			#[cfg(feature = "dag_cbor")]
			repos: world
				.with(|world: &mut World| {
					world.query::<&Pds>().iter(world).cloned().collect()
				})
				.await,
		}
	}

	/// The first of `sources` whose digest is not the one recorded, all read
	/// at once.
	async fn first_changed(
		&self,
		sources: &[ProvenanceSource],
	) -> Result<Option<ProvenanceSource>> {
		let unchanged = async_ext::try_join_all_bounded(
			BlobStore::GET_ALL_CONCURRENCY,
			sources.iter().map(|source| self.unchanged(source)),
		)
		.await?;
		sources
			.iter()
			.zip(unchanged)
			.find(|(_, unchanged)| !unchanged)
			.map(|(source, _)| source.clone())
			.xok()
	}

	/// Whether `source` still has the digest it was recorded with.
	async fn unchanged(&self, source: &ProvenanceSource) -> Result<bool> {
		match source {
			ProvenanceSource::File { path, cid } => {
				match self.store.exists(path).await? {
					true => Cid::raw(&self.store.get(path).await?) == *cid,
					false => false,
				}
			}
			ProvenanceSource::Record(strong_ref) => {
				self.record_unchanged(strong_ref).await?
			}
		}
		.xok()
	}

	/// Whether the record is still at the version recorded, `true` when no
	/// repo here holds it.
	#[cfg(feature = "dag_cbor")]
	async fn record_unchanged(&self, strong_ref: &StrongRef) -> Result<bool> {
		let uri = &strong_ref.uri;
		match self.repos.iter().find(|pds| pds.did() == uri.did()) {
			Some(pds) => pds
				.get_record(uri.collection(), uri.rkey())
				.await?
				.is_some_and(|entry| entry.cid == strong_ref.cid),
			None => true,
		}
		.xok()
	}

	/// No repo here holds a record without the data model.
	#[cfg(not(feature = "dag_cbor"))]
	async fn record_unchanged(&self, _strong_ref: &StrongRef) -> Result<bool> {
		true.xok()
	}
}

#[cfg(all(test, feature = "json"))]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;

	/// What the test derivation carries beside its provenance: the source's
	/// text, so a regeneration is visible in the file.
	#[derive(
		Debug, PartialEq, Reflect, serde::Serialize, serde::Deserialize,
	)]
	struct Echo(String);

	const SOURCE: &str = "notes.md";
	const DERIVED: &str = "derived/notes.json";

	/// A world declaring `derivation`, produced from [`SOURCE`] in a temp
	/// store holding `text`, the producer counting its runs.
	fn setup(
		derivation: Derivation,
		text: &str,
	) -> (World, Entity, BlobStore, Store<usize>) {
		let mut world = AsyncPlugin.into_world();
		let store = BlobStore::temp();
		let runs = Store::new(0);
		let entity = world
			.spawn((
				derivation,
				Action::<BlobStore, EntityBody>::new_async(
					move |cx: ActionContext<BlobStore>| async move {
						runs.set(runs.get() + 1);
						let bytes = cx.input.get(&RelPath::new(SOURCE)).await?;
						EntityBody::default()
							.with(&Provenance::new([ProvenanceSource::file(
								SOURCE, &bytes,
							)]))?
							.with(&Echo(str::from_utf8(&bytes)?.to_string()))
					},
				),
			))
			.id();
		let text = text.to_string();
		let inserted = store.clone();
		async_ext::block_on(async move {
			inserted.insert(&RelPath::new(SOURCE), text).await
		})
		.unwrap();
		(world, entity, store, runs)
	}

	/// [`Derivation::ensure`] in `world`'s async context.
	async fn ensure(
		world: &mut World,
		entity: Entity,
		store: &BlobStore,
	) -> Result<Freshness> {
		let store = store.clone();
		world
			.run_async_then(async move |world| {
				Derivation::ensure(&world, entity, &store).await
			})
			.await
	}

	/// The text the derived file holds.
	async fn echo(store: &BlobStore) -> String {
		Derivation::new(DERIVED)
			.read(store)
			.await
			.unwrap()
			.unwrap()
			.get::<Echo>()
			.unwrap()
			.0
	}

	#[beet_core::test]
	async fn writes_once_then_holds() {
		let (mut world, entity, store, runs) =
			setup(Derivation::new(DERIVED), "first");
		ensure(&mut world, entity, &store)
			.await
			.unwrap()
			.xpect_eq(Freshness::Missing);
		let written = store.get(&RelPath::new(DERIVED)).await.unwrap();
		// written pretty, so a committed one reads in a diff
		str::from_utf8(&written).unwrap().xpect_contains("\n");
		echo(&store).await.xpect_eq("first");
		ensure(&mut world, entity, &store)
			.await
			.unwrap()
			.xpect_eq(Freshness::Fresh);
		runs.get().xpect_eq(1);
		store
			.get(&RelPath::new(DERIVED))
			.await
			.unwrap()
			.xpect_eq(written);
	}

	#[beet_core::test]
	async fn regenerates_a_changed_source() {
		let (mut world, entity, store, runs) =
			setup(Derivation::new(DERIVED), "first");
		ensure(&mut world, entity, &store).await.unwrap();
		store.insert(&RelPath::new(SOURCE), "second").await.unwrap();
		ensure(&mut world, entity, &store)
			.await
			.unwrap()
			.to_string()
			.xpect_eq("file `notes.md` changed");
		echo(&store).await.xpect_eq("second");
		runs.get().xpect_eq(2);
	}

	#[beet_core::test]
	async fn regenerates_past_its_ttl() {
		let (mut world, entity, store, runs) = setup(
			Derivation::new(DERIVED).with_ttl(Duration::from_secs(60)),
			"first",
		);
		ensure(&mut world, entity, &store).await.unwrap();
		// the ttl rides the file, and an hour passes
		let derivation = Derivation::new(DERIVED);
		let mut body = derivation.read(&store).await.unwrap().unwrap();
		let provenance = body.get::<Provenance>().unwrap();
		provenance.ttl.xpect_eq(Some(Duration::from_secs(60)));
		body.insert(&Provenance {
			created: Timestamp::UNIX_EPOCH,
			..provenance
		})
		.unwrap();
		store
			.insert(
				&RelPath::new(DERIVED),
				MediaType::Json.serialize(&body).unwrap(),
			)
			.await
			.unwrap();
		ensure(&mut world, entity, &store)
			.await
			.unwrap()
			.xpect_eq(Freshness::Expired);
		runs.get().xpect_eq(2);
	}

	#[beet_core::test]
	async fn restores_a_deleted_or_hand_written_file() {
		let (mut world, entity, store, runs) =
			setup(Derivation::new(DERIVED), "first");
		ensure(&mut world, entity, &store).await.unwrap();
		store.remove(&RelPath::new(DERIVED)).await.unwrap();
		ensure(&mut world, entity, &store)
			.await
			.unwrap()
			.xpect_eq(Freshness::Missing);
		store.insert(&RelPath::new(DERIVED), "{}").await.unwrap();
		ensure(&mut world, entity, &store)
			.await
			.unwrap()
			.to_string()
			.xpect_contains("not a derived document");
		echo(&store).await.xpect_eq("first");
		runs.get().xpect_eq(3);
	}

	#[beet_core::test]
	async fn needs_a_producer_answering_provenance() {
		let mut world = AsyncPlugin.into_world();
		let store = BlobStore::temp();
		let bare = world.spawn(Derivation::new(DERIVED)).id();
		ensure(&mut world, bare, &store)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("no producing action");
		let unsourced = world
			.spawn((
				Derivation::new(DERIVED),
				Action::<BlobStore, EntityBody>::new_async(
					async |_: ActionContext<BlobStore>| EntityBody::default(),
				),
			))
			.id();
		ensure(&mut world, unsourced, &store)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("answered no `Provenance`");
	}

	/// A record source is checked against the repo holding its did, and one
	/// no repo here holds is left to the ttl.
	#[cfg(feature = "dag_cbor")]
	#[beet_core::test]
	async fn regenerates_a_changed_record() {
		let mut world = AsyncPlugin.into_world();
		let store = BlobStore::temp();
		let pds = Pds::temp();
		world.spawn(pds.clone());
		let collection = Nsid::new_static("org.example.note");
		let rkey = Rkey::parse("self").unwrap();
		let put = async |text: &str| {
			let body = value!({ "text": text }).try_into().unwrap();
			pds.put_record(&collection, Rkeyed::new(rkey.clone(), body))
				.await
				.unwrap()
		};
		let written = put("first").await;
		let elsewhere = StrongRef::new(
			AtUri::new(
				Did::parse("did:plc:bbbbbbbbbbbbbbbbbbbbbbbb").unwrap(),
				collection.clone(),
				rkey.clone(),
			),
			written.cid.clone(),
		);
		let sources = [
			ProvenanceSource::Record(written),
			ProvenanceSource::Record(elsewhere),
		];
		let entity = world
			.spawn((
				Derivation::new(DERIVED),
				Action::<BlobStore, EntityBody>::new_async(
					move |_: ActionContext<BlobStore>| {
						let sources = sources.clone();
						async move {
							EntityBody::default()
								.with(&Provenance::new(sources))
						}
					},
				),
			))
			.id();
		ensure(&mut world, entity, &store).await.unwrap();
		ensure(&mut world, entity, &store)
			.await
			.unwrap()
			.xpect_eq(Freshness::Fresh);
		put("second").await;
		ensure(&mut world, entity, &store)
			.await
			.unwrap()
			.to_string()
			.xpect_contains("org.example.note/self` changed");
	}
}
