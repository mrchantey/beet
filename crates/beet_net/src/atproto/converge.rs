//! Making a collection match what a document declares.
use crate::prelude::*;
use beet_core::prelude::*;

/// What a [`Pds::converge`] may do beyond writing the wanted records.
///
/// The defaults are the conservative ones: write what differs, report
/// orphans, delete nothing.
#[derive(SetWith)]
pub struct Converge<T> {
	/// Delete the orphans rather than only reporting them.
	prune: bool,
	/// Report what a run would do, writing and deleting nothing. The report's
	/// strong refs carry the cid each write would produce.
	dry_run: bool,
	/// Which listed records this converge answers for, so an unwanted one is
	/// an orphan rather than a stranger's. A collection is often shared: an
	/// author's reposts hold the ones they made by hand beside the ones a
	/// publish made, and only the latter are ever orphans.
	#[set_with(skip)]
	scope: Box<dyn Fn(&T) -> bool + Send + Sync>,
}

impl<T> Default for Converge<T> {
	fn default() -> Self {
		Self {
			prune: false,
			dry_run: false,
			scope: Box::new(|_| true),
		}
	}
}

impl<T> Converge<T> {
	/// Answer for only the listed records `scope` accepts; by default every
	/// record of the collection that reads as a `T`.
	pub fn with_scope(
		mut self,
		scope: impl 'static + Send + Sync + Fn(&T) -> bool,
	) -> Self {
		self.scope = Box::new(scope);
		self
	}
}

/// What one [`Pds::converge`] did, or under `dry_run` would do, each list in
/// the order the wanted records were given, then rkey order for the listing.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ConvergeReport {
	/// Wanted records with nothing at their rkey, written.
	pub created: Vec<StrongRef>,
	/// Wanted records that differed from what was there, rewritten.
	pub updated: Vec<StrongRef>,
	/// Wanted records already matching, left alone.
	pub unchanged: Vec<StrongRef>,
	/// Records in scope that nothing wanted, left in place.
	pub orphans: Vec<StrongRef>,
	/// Records in scope that nothing wanted, deleted under `prune`.
	pub removed: Vec<StrongRef>,
}

impl ConvergeReport {
	/// Whether the run wrote and deleted nothing.
	pub fn is_noop(&self) -> bool {
		self.created.is_empty()
			&& self.updated.is_empty()
			&& self.removed.is_empty()
	}
}

impl core::fmt::Display for ConvergeReport {
	/// One line per record, `created at://..`, the line a publish logs.
	fn fmt(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
		for (verb, refs) in [
			("created", &self.created),
			("updated", &self.updated),
			("unchanged", &self.unchanged),
			("orphan", &self.orphans),
			("removed", &self.removed),
		] {
			for strong_ref in refs {
				writeln!(formatter, "{verb:<9} {}", strong_ref.uri)?;
			}
		}
		Ok(())
	}
}

impl Pds {
	/// Make `T`'s collection match `wanted`, each record paired with the rkey
	/// it lives at: get it there, compare, and write only what differs; then
	/// list the collection once to find the orphans in `options`' scope,
	/// deleting them under `prune`.
	///
	/// Both sides are read through `T` before the comparison, so a field the
	/// PDS added that `T` does not know never reads as a change, and `T`'s
	/// `PartialEq` decides the rest. Running it
	/// twice writes nothing the second time, and an interrupted run resumes,
	/// since every record it already wrote now compares equal.
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// #[derive(Debug, PartialEq, Serialize, Deserialize)]
	/// struct Note {
	/// 	text: String,
	/// }
	/// impl AtprotoRecord for Note {
	/// 	const COLLECTION: Nsid = Nsid::new_static("com.example.note");
	/// }
	/// # async_ext::block_on(async {
	/// let pds = Pds::temp();
	/// let note = || Rkeyed::new(Rkey::parse("first").unwrap(), Note { text: "hi".into() });
	/// let report = pds.converge([note()], &default()).await?;
	/// report.created.len().xpect_eq(1);
	/// pds.converge([note()], &default()).await?.is_noop().xpect_true();
	/// # Ok::<_, BevyError>(())
	/// # })
	/// # .unwrap();
	/// ```
	pub async fn converge<T: AtprotoRecord>(
		&self,
		wanted: impl IntoIterator<Item = Rkeyed<T>>,
		options: &Converge<T>,
	) -> Result<ConvergeReport> {
		let mut report = ConvergeReport::default();
		let mut wanted_rkeys = HashSet::<Rkey>::default();
		for record in wanted {
			let rkey = record.rkey().clone();
			if !wanted_rkeys.insert(rkey.clone()) {
				bevybail!(
					"two wanted `{}` records share the rkey `{rkey}`",
					T::COLLECTION
				);
			}
			let existing = self.get_record(&T::COLLECTION, &rkey).await?;
			let (list, changed) = match &existing {
				None => (&mut report.created, true),
				Some(existing) => {
					match Self::same(&*record, &existing.value)? {
						true => (&mut report.unchanged, false),
						false => (&mut report.updated, true),
					}
				}
			};
			let strong_ref = match (changed, options.dry_run, existing) {
				(false, _, Some(existing)) => existing.strong_ref(),
				(_, true, _) => StrongRef::new(
					self.uri(&T::COLLECTION, &rkey),
					dag_cbor_ext::record_cid(
						&AtprotoValue::from_serde(&*record)?
							.into_record(&T::COLLECTION)?,
					)?,
				),
				_ => self.put(&record).await?,
			};
			list.push(strong_ref);
		}
		// the one listing: records in scope that nothing wanted
		for entry in self.list_records(&T::COLLECTION).await? {
			if wanted_rkeys.contains(entry.rkey())
				|| !entry
					.value
					.clone()
					.into_serde::<T>()
					.is_ok_and(|record| (options.scope)(&record))
			{
				continue;
			}
			if !options.prune {
				report.orphans.push(entry.strong_ref());
				continue;
			}
			if !options.dry_run {
				self.delete_record(&T::COLLECTION, entry.rkey()).await?;
			}
			report.removed.push(entry.strong_ref());
		}
		report.xok()
	}

	/// Whether `existing` already holds `wanted`, both read through `T`. A
	/// body that does not read as a `T` at all differs.
	fn same<T: AtprotoRecord>(
		wanted: &T,
		existing: &AtprotoValue,
	) -> Result<bool> {
		let Ok(existing) = existing.clone().into_serde::<T>() else {
			return Ok(false);
		};
		AtprotoValue::from_serde(wanted)?
			.into_serde::<T>()?
			.xmap(|wanted| wanted == existing)
			.xok()
	}
}

#[cfg(test)]
pub(crate) mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A foreign record keyed by TID, its natural key `path`, with a float
	/// to cross the data model.
	#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
	struct Doc {
		path: String,
		title: String,
		weight: f32,
	}

	impl AtprotoRecord for Doc {
		const COLLECTION: Nsid = Nsid::new_static("com.example.doc");
	}

	/// `rkey` paired with its document.
	fn doc(rkey: &str, path: &str, title: &str) -> Rkeyed<Doc> {
		Rkeyed::new(Rkey::parse(rkey).unwrap(), Doc {
			path: path.into(),
			title: title.into(),
			weight: 0.1,
		})
	}

	/// The rkeys of `refs`, for a compact assertion.
	fn rkeys(refs: &[StrongRef]) -> Vec<String> {
		refs.iter()
			.map(|strong_ref| strong_ref.uri.rkey().to_string())
			.collect()
	}

	/// The converge suite, run against any repo: the emulator here, the
	/// throwaway account behind the live gate.
	pub(crate) async fn converge_suite(pds: &Pds) {
		let both = || [doc("aaa", "/one", "One"), doc("bbb", "/two", "Two")];
		// first write
		let report = pds.converge(both(), &default()).await.unwrap();
		rkeys(&report.created).xpect_eq(vec!["aaa", "bbb"]);
		// no-op
		let report = pds.converge(both(), &default()).await.unwrap();
		report.is_noop().xpect_true();
		rkeys(&report.unchanged).xpect_eq(vec!["aaa", "bbb"]);
		// update, its cid moving with its content
		let before =
			pds.get::<Doc>(&Rkey::parse("bbb").unwrap()).await.unwrap();
		let report = pds
			.converge(
				[doc("aaa", "/one", "One"), doc("bbb", "/two", "Two!")],
				&default(),
			)
			.await
			.unwrap();
		rkeys(&report.updated).xpect_eq(vec!["bbb"]);
		(report.updated[0].cid != before.unwrap().cid).xpect_true();
		// orphan: reported, left in place
		let report = pds
			.converge([doc("aaa", "/one", "One")], &default())
			.await
			.unwrap();
		rkeys(&report.orphans).xpect_eq(vec!["bbb"]);
		pds.get::<Doc>(&Rkey::parse("bbb").unwrap())
			.await
			.unwrap()
			.xpect_some();
		// a dry run prunes nothing but says what it would
		let report = pds
			.converge(
				[doc("aaa", "/one", "One")],
				&Converge::default().with_prune(true).with_dry_run(true),
			)
			.await
			.unwrap();
		rkeys(&report.removed).xpect_eq(vec!["bbb"]);
		pds.get::<Doc>(&Rkey::parse("bbb").unwrap())
			.await
			.unwrap()
			.xpect_some();
		// prune
		let report = pds
			.converge(
				[doc("aaa", "/one", "One")],
				&Converge::default().with_prune(true),
			)
			.await
			.unwrap();
		rkeys(&report.removed).xpect_eq(vec!["bbb"]);
		pds.list::<Doc>().await.unwrap().len().xpect_eq(1);
		// clean up, so a live account is left as it was found
		pds.converge::<Doc>([], &Converge::default().with_prune(true))
			.await
			.unwrap();
	}

	#[beet_core::test]
	async fn converges() { converge_suite(&Pds::temp()).await; }

	/// A dry run's report carries the cid the write would produce.
	#[beet_core::test]
	async fn a_dry_run_writes_nothing() {
		let pds = Pds::temp();
		let wanted = doc("aaa", "/one", "One");
		let dry = pds
			.converge([wanted.clone()], &Converge::default().with_dry_run(true))
			.await
			.unwrap();
		pds.list::<Doc>().await.unwrap().len().xpect_eq(0);
		pds.converge([wanted], &default())
			.await
			.unwrap()
			.created
			.xpect_eq(dry.created);
	}

	/// An interrupted run resumes: what it wrote compares equal, the rest is
	/// created, and nothing is written twice.
	#[beet_core::test]
	async fn resumes_an_interrupted_run() {
		let pds = Pds::temp();
		let all = || {
			["aaa", "bbb", "ccc"]
				.map(|rkey| doc(rkey, &format!("/{rkey}"), rkey))
		};
		let [first, ..] = all();
		pds.converge([first], &default()).await.unwrap();
		let report = pds.converge(all(), &default()).await.unwrap();
		rkeys(&report.unchanged).xpect_eq(vec!["aaa"]);
		rkeys(&report.created).xpect_eq(vec!["bbb", "ccc"]);
	}

	/// A lost index is rebuilt from one listing, matching each record by its
	/// natural key.
	#[beet_core::test]
	async fn rebuilds_an_index_from_a_listing() {
		let pds = Pds::temp();
		pds.converge(
			[doc("aaa", "/one", "One"), doc("bbb", "/two", "Two")],
			&default(),
		)
		.await
		.unwrap();
		pds.list::<Doc>()
			.await
			.unwrap()
			.into_iter()
			.map(|entry| (entry.value.path.clone(), entry.rkey().to_string()))
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				("/one".to_string(), "aaa".to_string()),
				("/two".to_string(), "bbb".to_string()),
			]);
	}

	/// A shared collection's strangers are never orphans, so never pruned.
	#[beet_core::test]
	async fn prunes_only_its_scope() {
		let pds = Pds::temp();
		pds.converge(
			[doc("aaa", "/blog/one", "One"), doc("bbb", "/notes/x", "X")],
			&default(),
		)
		.await
		.unwrap();
		let report = pds
			.converge::<Doc>(
				[],
				&Converge::default()
					.with_prune(true)
					.with_scope(|doc: &Doc| doc.path.starts_with("/blog/")),
			)
			.await
			.unwrap();
		rkeys(&report.removed).xpect_eq(vec!["aaa"]);
		pds.list::<Doc>().await.unwrap().len().xpect_eq(1);
	}

	#[beet_core::test]
	async fn refuses_a_shared_rkey() {
		Pds::temp()
			.converge(
				[doc("aaa", "/one", "One"), doc("aaa", "/two", "Two")],
				&default(),
			)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("share the rkey");
	}
}
