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
/// that same store, so a derivation ensured into a deploy's assembled copy
/// reads that copy) and answers the document: an [`EntityBody`] carrying a
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
/// 				EntityBody::default().with_typed(&Provenance::new([
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
/// A producer whose answer is binary (an image, an archive) writes those files
/// beside its document through [`DerivationOutputs::write`] and answers the
/// listing in the body. The document is then the set's manifest: a listed file
/// deleted or altered is stale like a changed source, and a file an earlier
/// generation listed but this one does not is removed, so a renamed output
/// leaves no orphan behind.
///
/// A derivation is declared by whatever owns its file, beside that file's
/// producing action, and ensured by a step gathering every one a document
/// declares, so nothing regenerates until something asks.
///
/// A record source is read from the repo of the `Pds` in the world holding its
/// did. A record no repo here holds cannot be seen from here and counts as
/// unchanged, so a derivation reading one declares a `ttl`. A url source (a
/// remote image a card embeds) counts as unchanged the same way, since nothing
/// local can hash it without fetching it again; one that practically never
/// changes, like a video's poster frame, needs no `ttl`.
///
/// What a producer reads off the world rather than from a file (a title a page
/// declares in code, a theme colour) the declaration computes itself, as
/// [`values`](Self::values) it keeps current. Each regeneration stamps them
/// onto its provenance, so the file is stale once the declaration's values
/// differ from the ones it was computed from, a value added or dropped
/// included.
///
/// A file whose producer reads what a deploy must never depend on (a listing
/// of a remote repo, say) names the verb that alone writes it
/// ([`written_by`](Self::written_by)): an ensure step leaves it as committed,
/// `beet check` names the verb for a missing or stale one, and the verb writes
/// it with [`Derivation::rewrite`], whatever its freshness.
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct Derivation {
	/// The derived file's path in its store, its extension naming the format.
	pub path: RelPath,
	/// How long a regeneration holds for an input nothing local can hash,
	/// stamped onto the [`Provenance`] it writes.
	pub ttl: Option<Duration>,
	/// The file sources the declaration itself names, so a file computed from
	/// another (a declaration edited to a new source) is stale.
	pub inputs: Vec<RelPath>,
	/// The values the declaration computes from the world, by name and
	/// digest, stamped onto each regeneration's [`Provenance`].
	pub values: Vec<ProvenanceValue>,
	/// The verb that alone writes this file, ie `publish`, or `None` when any
	/// ensure step may: `<Derive/>` skips a file one names, so a deploy
	/// publishes it exactly as committed.
	pub written_by: Option<SmolStr>,
}

impl Derivation {
	/// The derived file at `path`, holding until a source changes.
	pub fn new(path: impl Into<RelPath>) -> Self {
		Self {
			path: path.into(),
			ttl: None,
			inputs: Vec::new(),
			values: Vec::new(),
			written_by: None,
		}
	}

	/// This derivation declaring `input` as a file source it reads.
	pub fn with_input(mut self, input: impl Into<RelPath>) -> Self {
		self.inputs.push(input.into());
		self
	}

	/// This derivation declaring `value`, computed from the world.
	pub fn with_value(mut self, value: ProvenanceValue) -> Self {
		self.values.push(value);
		self
	}

	/// This derivation written by `verb` alone, see
	/// [`written_by`](Self::written_by).
	pub fn with_written_by(mut self, verb: impl Into<SmolStr>) -> Self {
		self.written_by = Some(verb.into());
		self
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
			derivation
				.regenerate(world.entity(entity), store, false)
				.await?;
			info!(
				"derived file `{}` regenerated: {freshness}",
				derivation.path
			);
		}
		freshness.xok()
	}

	/// Regenerate the derived file `entity` declares in `store` whatever its
	/// freshness, which is how the verb a file is
	/// [`written_by`](Self::written_by) writes it, since such a file's
	/// provenance may name nothing that could go stale. A document matching
	/// the one in place but for its creation time leaves the file untouched,
	/// so running the verb twice changes nothing. Answers whether it wrote.
	pub async fn rewrite(
		world: &AsyncWorld,
		entity: Entity,
		store: &BlobStore,
	) -> Result<bool> {
		world
			.entity(entity)
			.get_cloned::<Self>()
			.await?
			.regenerate(world.entity(entity), store, true)
			.await
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
		let (provenance, outputs) = match self
			.media_type()?
			.deserialize::<EntityBody>(&bytes)
			.and_then(|body| {
				(
					body.get_typed::<Provenance>()?,
					DerivationOutputs::of(&body)?,
				)
					.xok()
			}) {
			Ok(read) => read,
			Err(error) => return Freshness::Invalid(error.to_string()).xok(),
		};
		if provenance.expired(Timestamp::now()) {
			return Freshness::Expired.xok();
		}
		if let Some(input) = self.inputs.iter().find(|input| {
			!provenance.sources.iter().any(|source| {
				matches!(source, ProvenanceSource::File { path, .. } if path == *input)
			})
		}) {
			return Freshness::Redeclared(input.clone()).xok();
		}
		if let Some(name) = self.first_revalued(&provenance) {
			return Freshness::Revalued(name).xok();
		}
		if let Some(source) = digests.first_changed(&provenance.sources).await?
		{
			return Freshness::Changed(source).xok();
		}
		outputs
			.first_stale(store)
			.await?
			.unwrap_or(Freshness::Fresh)
			.xok()
	}

	/// The name of the first value declared and not computed from, or
	/// computed from and no longer declared, at the digest each names.
	fn first_revalued(&self, provenance: &Provenance) -> Option<SmolStr> {
		let computed = provenance
			.sources
			.iter()
			.filter_map(|source| match source {
				ProvenanceSource::Value(value) => Some(value),
				_ => None,
			})
			.collect::<Vec<_>>();
		self.values
			.iter()
			.find(|value| !computed.contains(value))
			.or_else(|| {
				computed
					.iter()
					.find(|value| !self.values.contains(value))
					.copied()
			})
			.map(|value| value.name.clone())
	}

	/// Run the producing action and write the document it answers, stamped
	/// with the time, this declaration's ttl and its values, removing every
	/// output the document it replaces listed and this one does not. With
	/// `keep_unchanged`, a document equal to the one in place but for its
	/// creation time is not written. Answers whether it wrote.
	async fn regenerate(
		&self,
		entity: AsyncEntity,
		store: &BlobStore,
		keep_unchanged: bool,
	) -> Result<bool> {
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
		// a previous document that no longer reads lists nothing to remove
		let previous = self.read(store).await.ok().flatten();
		let superseded = previous
			.as_ref()
			.map(DerivationOutputs::of)
			.transpose()?
			.unwrap_or_default();
		let mut body =
			entity.call::<BlobStore, EntityBody>(store.clone()).await?;
		if !body.contains_typed::<Provenance>() {
			bevybail!(
				"the producing action of derived file `{}` answered no \
				 `Provenance`: a derived file names the sources it was computed from",
				self.path
			);
		}
		let produced = body.get_typed::<Provenance>()?;
		let mut provenance = Provenance {
			created: Timestamp::now(),
			ttl: self.ttl,
			sources: produced
				.sources
				.into_iter()
				.filter(|source| !matches!(source, ProvenanceSource::Value(_)))
				.chain(self.values.iter().cloned().map(ProvenanceSource::Value))
				.collect(),
		};
		// unchanged but for when it was made: keep the file in place
		if keep_unchanged
			&& let Some(previous) = &previous
			&& let Ok(previous_provenance) = previous.get_typed::<Provenance>()
		{
			provenance.created = previous_provenance.created;
			body.insert_typed(&provenance)?;
			if self.encode(&body)? == self.encode(previous)? {
				return Ok(false);
			}
			provenance.created = Timestamp::now();
		}
		body.insert_typed(&provenance)?;
		store.insert(&self.path, self.encode(&body)?).await?;
		let outputs = DerivationOutputs::of(&body)?;
		for output in superseded
			.0
			.iter()
			.filter(|old| outputs.get(&old.path).is_none())
		{
			if store.exists(&output.path).await? {
				store.remove(&output.path).await?;
			}
		}
		Ok(true)
	}

	/// `body` in the format the path's extension names, pretty where the
	/// format has one.
	fn encode(&self, body: &EntityBody) -> Result<Vec<u8>> {
		self.media_type()?
			.serialize_with_options(body, SerializeOptions { pretty: true })
	}
}

/// The files a derived document's producer wrote beside it, each with the cid
/// it was written with, carried in the document so a deleted or altered one
/// counts as stale.
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// # async fn run() -> Result {
/// let store = BlobStore::temp();
/// let mut outputs = DerivationOutputs::default();
/// outputs.write(&store, "icons/icon-16.png", vec![0u8; 4]).await?;
/// let body = EntityBody::default().with_typed(&outputs)?;
/// # let _ = body;
/// # Ok(()) }
/// ```
#[derive(
	Debug, Default, Clone, PartialEq, Eq, Reflect, Serialize, Deserialize,
)]
#[reflect(Default, Serialize, Deserialize)]
pub struct DerivationOutputs(pub Vec<DerivationOutput>);

/// One file a producer wrote beside its derived document.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
pub struct DerivationOutput {
	/// The file's path in the store its document is in.
	pub path: RelPath,
	/// Its content id as written.
	pub cid: Cid,
}

impl DerivationOutputs {
	/// The outputs `body` lists, none when it lists none.
	pub fn of(body: &EntityBody) -> Result<Self> {
		match body.contains_typed::<Self>() {
			true => body.get_typed::<Self>(),
			false => Self::default().xok(),
		}
	}

	/// Write `bytes` at `path` in `store` and list it.
	pub async fn write(
		&mut self,
		store: &BlobStore,
		path: impl Into<RelPath>,
		bytes: impl Into<Vec<u8>>,
	) -> Result {
		let path = path.into();
		let bytes = bytes.into();
		let cid = Cid::raw(&bytes);
		store.insert(&path, bytes).await?;
		self.0.retain(|output| output.path != path);
		self.0.push(DerivationOutput { path, cid });
		Ok(())
	}

	/// The listed output at `path`.
	pub fn get(&self, path: &RelPath) -> Option<&DerivationOutput> {
		self.0.iter().find(|output| &output.path == path)
	}

	/// The first listed output that is missing from `store` or no longer has
	/// the cid it was written with, all read at once.
	async fn first_stale(
		&self,
		store: &BlobStore,
	) -> Result<Option<Freshness>> {
		let found = async_ext::try_join_all_bounded(
			BlobStore::GET_ALL_CONCURRENCY,
			self.0
				.iter()
				.map(async |output| -> Result<Option<Freshness>> {
					if !store.exists(&output.path).await? {
						return Some(Freshness::OutputMissing(
							output.path.clone(),
						))
						.xok();
					}
					(Cid::raw(&store.get(&output.path).await?) != output.cid)
						.then(|| Freshness::OutputChanged(output.path.clone()))
						.xok()
				}),
		)
		.await?;
		found.into_iter().flatten().next().xok()
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
	/// The declaration names a file source the file was not computed from.
	Redeclared(RelPath),
	/// A value the declaration computes differs from the one the file was
	/// computed from, or only one of them has it.
	Revalued(SmolStr),
	/// A file the document lists as written beside it is gone.
	OutputMissing(RelPath),
	/// A file the document lists as written beside it was altered.
	OutputChanged(RelPath),
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
			Self::Redeclared(path) => {
				write!(f, "not computed from the declared `{path}`")
			}
			Self::Revalued(name) => {
				write!(f, "value `{name}` changed")
			}
			Self::OutputMissing(path) => {
				write!(f, "output `{path}` is missing")
			}
			Self::OutputChanged(path) => {
				write!(f, "output `{path}` was altered")
			}
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
			// see `Derivation`'s docs: a url is never fetched to be checked
			ProvenanceSource::Url { .. } => true,
			// the declaration compares its values, see `first_revalued`
			ProvenanceSource::Value(_) => true,
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
							.with_typed(&Provenance::new([
								ProvenanceSource::file(SOURCE, &bytes),
							]))?
							.with_typed(&Echo(
								str::from_utf8(&bytes)?.to_string(),
							))
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
			.get_typed::<Echo>()
			.unwrap()
			.0
	}

	/// [`Derivation::rewrite`] in `world`'s async context.
	async fn rewrite(
		world: &mut World,
		entity: Entity,
		store: &BlobStore,
	) -> Result<bool> {
		let store = store.clone();
		world
			.run_async_then(async move |world| {
				Derivation::rewrite(&world, entity, &store).await
			})
			.await
	}

	/// A rewrite runs the producer whatever the freshness, and leaves a file
	/// it would only restamp byte for byte as it was.
	#[beet_core::test]
	async fn rewrites_only_a_changed_document() {
		let (mut world, entity, store, runs) =
			setup(Derivation::new(DERIVED), "first");
		rewrite(&mut world, entity, &store)
			.await
			.unwrap()
			.xpect_true();
		let written = store.get(&RelPath::new(DERIVED)).await.unwrap();
		rewrite(&mut world, entity, &store)
			.await
			.unwrap()
			.xpect_false();
		runs.get().xpect_eq(2);
		store
			.get(&RelPath::new(DERIVED))
			.await
			.unwrap()
			.xpect_eq(written);
		store.insert(&RelPath::new(SOURCE), "second").await.unwrap();
		rewrite(&mut world, entity, &store)
			.await
			.unwrap()
			.xpect_true();
		echo(&store).await.xpect_eq("second");
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
		let provenance = body.get_typed::<Provenance>().unwrap();
		provenance.ttl.xpect_eq(Some(Duration::from_secs(60)));
		body.insert_typed(&Provenance {
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

	/// A declaration edited to name another source regenerates.
	#[beet_core::test]
	async fn regenerates_a_redeclared_input() {
		let (mut world, entity, store, runs) =
			setup(Derivation::new(DERIVED), "first");
		ensure(&mut world, entity, &store).await.unwrap();
		world
			.entity_mut(entity)
			.insert(Derivation::new(DERIVED).with_input("other.md"));
		ensure(&mut world, entity, &store)
			.await
			.unwrap()
			.xpect_eq(Freshness::Redeclared(RelPath::new("other.md")));
		runs.get().xpect_eq(2);
	}

	/// A declared value is stamped onto the provenance, and regenerates once
	/// it changes or is dropped.
	#[beet_core::test]
	async fn regenerates_a_changed_value() {
		let declared = |title: &str| {
			Derivation::new(DERIVED)
				.with_value(ProvenanceValue::new("title", title.as_bytes()))
		};
		let (mut world, entity, store, runs) = setup(declared("first"), "text");
		ensure(&mut world, entity, &store).await.unwrap();
		ensure(&mut world, entity, &store)
			.await
			.unwrap()
			.xpect_eq(Freshness::Fresh);
		world.entity_mut(entity).insert(declared("second"));
		ensure(&mut world, entity, &store)
			.await
			.unwrap()
			.xpect_eq(Freshness::Revalued("title".into()));
		world.entity_mut(entity).insert(Derivation::new(DERIVED));
		ensure(&mut world, entity, &store)
			.await
			.unwrap()
			.xpect_eq(Freshness::Revalued("title".into()));
		ensure(&mut world, entity, &store)
			.await
			.unwrap()
			.xpect_eq(Freshness::Fresh);
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

	/// A derivation whose producer writes `<source text>.bin` beside its
	/// document, so a changed source renames the output.
	fn with_outputs() -> (World, Entity, BlobStore) {
		let mut world = AsyncPlugin.into_world();
		let store = BlobStore::temp();
		let entity = world
			.spawn((
				Derivation::new(DERIVED),
				Action::<BlobStore, EntityBody>::new_async(
					async |cx: ActionContext<BlobStore>| {
						let bytes = cx.input.get(&RelPath::new(SOURCE)).await?;
						let mut outputs = DerivationOutputs::default();
						outputs
							.write(
								&cx.input,
								format!("{}.bin", str::from_utf8(&bytes)?),
								bytes.to_vec(),
							)
							.await?;
						EntityBody::default()
							.with_typed(&Provenance::new([
								ProvenanceSource::file(SOURCE, &bytes),
							]))?
							.with_typed(&outputs)
					},
				),
			))
			.id();
		(world, entity, store)
	}

	#[beet_core::test]
	async fn restores_its_outputs() {
		let (mut world, entity, store) = with_outputs();
		store.insert(&RelPath::new(SOURCE), "first").await.unwrap();
		ensure(&mut world, entity, &store).await.unwrap();
		let output = RelPath::new("first.bin");
		store.remove(&output).await.unwrap();
		ensure(&mut world, entity, &store)
			.await
			.unwrap()
			.xpect_eq(Freshness::OutputMissing(output.clone()));
		store.insert(&output, "tampered").await.unwrap();
		ensure(&mut world, entity, &store)
			.await
			.unwrap()
			.xpect_eq(Freshness::OutputChanged(output.clone()));
		store
			.get(&output)
			.await
			.unwrap()
			.to_vec()
			.xpect_eq(b"first".to_vec());
		ensure(&mut world, entity, &store)
			.await
			.unwrap()
			.xpect_eq(Freshness::Fresh);
	}

	/// An output the new generation no longer lists is removed.
	#[beet_core::test]
	async fn removes_superseded_outputs() {
		let (mut world, entity, store) = with_outputs();
		store.insert(&RelPath::new(SOURCE), "first").await.unwrap();
		ensure(&mut world, entity, &store).await.unwrap();
		store.insert(&RelPath::new(SOURCE), "second").await.unwrap();
		ensure(&mut world, entity, &store).await.unwrap();
		store
			.exists(&RelPath::new("first.bin"))
			.await
			.unwrap()
			.xpect_false();
		store
			.exists(&RelPath::new("second.bin"))
			.await
			.unwrap()
			.xpect_true();
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
								.with_typed(&Provenance::new(sources))
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
