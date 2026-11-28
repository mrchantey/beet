//! [`PackageReleaser`]: a package record frozen as a release in the package's
//! space, and a release published to the account's public repo.
use super::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// The two verbs after a compile. *Release* freezes a package record under its
/// version in the package's space: its body copied whole into a
/// [`ReleaseRecord`], created and never put, so a version cannot be
/// rewritten. *Publish* copies a release from the space to the account's
/// public repo byte for byte, with every scene it names and every blob, so
/// the copy's cid is the space's and the two are known to agree by comparing
/// cids. A release never published stays private.
///
/// Both work through one scoped [`Pds`], the account's, reading a space or
/// the public repo by the scope a record's uri names, and a dry run is the
/// same run over [`Pds::dry`]. Both report each write as a converge does, so
/// a second run reads `unchanged` throughout.
///
/// A path dependency within the same account is released first, and the
/// release pins it to that release, its did and its version (design item
/// 25); a publish publishes it first, so a published release never depends
/// on a private one. A dependency by address is resolved by the dependency
/// resolver, which pins it to a release itself.
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// # use beet_scene::prelude::*;
/// # async_ext::block_on(async {
/// let source = BlobStore::temp();
/// source
/// 	.insert(&fwd_path!("todos/main.bsx"), r#"<Package name="todos" version="1.0.0"/>"#)
/// 	.await?;
/// let pds = PackageCompiler::local_pds(&source);
/// let compiled = PackageCompiler::new(DocParsers::of(&ScenePlugin.into_world()), pds.clone())
/// 	.compile(&source, &fwd_path!("todos/main.bsx"))
/// 	.await?;
/// let releaser = PackageReleaser::new(pds);
/// let release = releaser.release(&compiled.package_record).await?;
/// release.record.uri.to_string().xpect_contains("/org.beet.release/todos:1.0.0");
/// let published = releaser.publish(&release.record, false).await?;
/// published.record.cid.xpect_eq(release.record.cid);
/// # Ok::<_, BevyError>(())
/// # })
/// # .unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct PackageReleaser {
	pds: Pds,
}

impl PackageReleaser {
	/// The rkey suffix of a package's newest release with no prerelease,
	/// `<package>:latest`, the one release rkey written by put.
	pub const LATEST: &'static str = "latest";

	/// A releaser over the account `pds` is a repo of.
	pub fn new(pds: Pds) -> Self { Self { pds } }

	/// The rkey of `package`'s `latest` release.
	pub fn latest_rkey(package: &PackageName) -> Rkey {
		Rkey::parse(&format!("{package}:{}", Self::LATEST))
			.expect("a package name is rkey safe")
	}

	/// Freeze the package record `package_record` names as a release at its
	/// version, in the space it lives in. A version already released with
	/// the same cid is `unchanged`; one released with another cid is refused,
	/// naming the version and both cids. A version with no prerelease newer
	/// than the current `latest` is then copied whole to `latest`.
	///
	/// ## Errors
	/// Errors when the package record moved past `package_record`, when a
	/// path dependency is unresolved or of another account, and on the
	/// refusal above.
	pub fn release<'a>(
		&'a self,
		package_record: &'a StrongRef,
	) -> LifetimeSendBoxedFuture<'a, Result<ReleaseReport>> {
		Box::pin(async move {
			let space = self.pds.scoped(package_record.uri.scope().clone())?;
			let entry = Self::read_exact(
				&space,
				&PackageRecord::COLLECTION,
				package_record,
			)
			.await?;
			let mut body = EntityBodyRecord::try_from(entry.value)?;
			let package = body.get::<Package>()?;

			// path dependencies first, each pinned to its release
			let mut dependencies = Vec::new();
			if let Some(mut declared) = body.try_get::<Dependencies>()? {
				for (key, dependency) in declared.0.iter_mut() {
					if dependency.package.is_some() {
						continue;
					}
					let resolved =
						self.path_dependency(&package, key, dependency)?;
					let released = self.release(&resolved.record).await?;
					dependency.resolved = Some(ResolvedDependency {
						did: resolved.did.clone(),
						version: released.package.version.clone(),
						record: released.record.clone(),
					});
					dependencies.push(released);
				}
				body.insert(&declared)?;
			}

			// the body frozen, created where no release holds its version
			let release = ReleaseRecord::accept(
				body.with_collection(ReleaseRecord::COLLECTION),
			)?;
			let rkey = release.rkey().clone();
			let value = AtprotoValue::from(release.into_body());
			let mut writes = ConvergeReport {
				dry_run: space.is_dry_run(),
				..default()
			};
			let record = match space
				.get_record(&ReleaseRecord::COLLECTION, &rkey)
				.await?
			{
				Some(existing) if existing.cid == value.cid() => {
					writes.unchanged.push(existing.strong_ref());
					existing.strong_ref()
				}
				Some(existing) => bevybail!(
					"`{}@{}` is already released as {}, and releasing it \
					 again would write {}: a release is never rewritten, so \
					 give the package a new version",
					package.name,
					package.version,
					existing.cid,
					value.cid()
				),
				None => {
					let created = space
						.put_record_if(
							&ReleaseRecord::COLLECTION,
							Rkeyed::new(rkey.clone(), value.clone()),
							Precondition::Absent,
						)
						.await?;
					writes.created.push(created.clone());
					created
				}
			};
			Self::converge_latest(&space, &space, &package, value, &mut writes)
				.await?;
			ReleaseReport {
				package,
				record,
				writes,
				dependencies,
			}
			.xok()
		})
	}

	/// Copy the release `release` names, in its package's space, to the
	/// account's public repo: every scene record its [`Scenes`] names, then
	/// the release with every blob its [`Files`] and its scenes reference,
	/// then `latest` where it is the newest with no prerelease. Each is
	/// written only where the public copy is missing or behind, so a second
	/// run is `unchanged` throughout. `as_home` writes the account's
	/// [`HomeRecord`] naming this package, replacing one naming another,
	/// which nothing else does.
	///
	/// ## Errors
	/// Errors when the release, or a scene it names, is missing or no longer
	/// has the cid it was named by.
	pub fn publish<'a>(
		&'a self,
		release: &'a StrongRef,
		as_home: bool,
	) -> LifetimeSendBoxedFuture<'a, Result<ReleaseReport>> {
		Box::pin(async move {
			let space = self.pds.scoped(release.uri.scope().clone())?;
			let public = self.pds.public()?;
			let entry =
				Self::read_exact(&space, &ReleaseRecord::COLLECTION, release)
					.await?;
			let body = EntityBodyRecord::try_from(entry.value.clone())?;
			let package = body.get::<Package>()?;

			// path dependencies first, so nothing published depends on a
			// private release
			let mut dependencies = Vec::new();
			for (key, dependency) in body
				.try_get::<Dependencies>()?
				.unwrap_or_default()
				.0
				.iter()
				.filter(|(_, dependency)| dependency.package.is_none())
			{
				let resolved =
					self.path_dependency(&package, key, dependency)?;
				dependencies.push(self.publish(&resolved.record, false).await?);
			}

			let mut writes = ConvergeReport {
				dry_run: public.is_dry_run(),
				..default()
			};
			let mut copied = HashSet::<Rkey>::default();
			for scene in
				body.try_get::<Scenes>()?.unwrap_or_default().0.values()
			{
				if !copied.insert(scene.record.uri.rkey().clone()) {
					continue;
				}
				let scene = Self::read_exact(
					&space,
					&SceneRecord::COLLECTION,
					&scene.record,
				)
				.await?;
				public
					.converge_copy(
						&space,
						&SceneRecord::COLLECTION,
						scene.into_rkeyed(),
						&mut writes,
					)
					.await?;
			}
			let value = entry.value.clone();
			let record = public
				.converge_copy(
					&space,
					&ReleaseRecord::COLLECTION,
					entry.into_rkeyed(),
					&mut writes,
				)
				.await?;
			Self::converge_latest(
				&public,
				&space,
				&package,
				value,
				&mut writes,
			)
			.await?;
			if as_home {
				Self::converge_home(&public, &package.name, &mut writes)
					.await?;
			}
			ReleaseReport {
				package,
				record,
				writes,
				dependencies,
			}
			.xok()
		})
	}

	/// The record at `strong_ref` in `repo`, refused when missing or when its
	/// cid is no longer the one it was named by.
	async fn read_exact(
		repo: &Pds,
		collection: &Nsid,
		strong_ref: &StrongRef,
	) -> Result<RecordEntry> {
		let entry = repo
			.get_record(collection, strong_ref.uri.rkey())
			.await?
			.ok_or_else(|| {
			bevyhow!("{} holds no record", strong_ref.uri)
		})?;
		if entry.cid != strong_ref.cid {
			bevybail!(
				"{} was named by {} but is now {}",
				strong_ref.uri,
				strong_ref.cid,
				entry.cid
			);
		}
		entry.xok()
	}

	/// What the path dependency `key` of `package` resolved to, refused
	/// when it is unresolved or of another account (design item 25).
	fn path_dependency<'a>(
		&self,
		package: &Package,
		key: &PackageName,
		dependency: &'a Dependency,
	) -> Result<&'a ResolvedDependency> {
		let Some(resolved) = &dependency.resolved else {
			bevybail!(
				"`{}` depends on `{key}` by path alone and it is unresolved: \
				 compile the package first",
				package.name
			);
		};
		if &resolved.did != self.pds.did() {
			bevybail!(
				"`{}` depends on `{key}` by path alone, a package of {} rather \
				 than {}: a release depends on another account's package by \
				 address",
				package.name,
				resolved.did,
				self.pds.did()
			);
		}
		resolved.xok()
	}

	/// Copy the release `value` of `package` to `latest` in `repo`, its blobs
	/// read from `from`, unless it is a prerelease or `latest` already holds
	/// a newer version.
	async fn converge_latest(
		repo: &Pds,
		from: &Pds,
		package: &Package,
		value: AtprotoValue,
		writes: &mut ConvergeReport,
	) -> Result {
		if package.version.is_prerelease() {
			return Ok(());
		}
		let rkey = Self::latest_rkey(&package.name);
		if let Some(latest) = repo.get::<ReleaseRecord>(&rkey).await?
			&& latest.value.body().get::<Package>()?.version > package.version
		{
			return Ok(());
		}
		repo.converge_copy(
			from,
			&ReleaseRecord::COLLECTION,
			Rkeyed::new(rkey, value),
			writes,
		)
		.await?;
		Ok(())
	}

	/// Write `public`'s home record naming `package`, left alone where it
	/// already does.
	async fn converge_home(
		public: &Pds,
		package: &PackageName,
		writes: &mut ConvergeReport,
	) -> Result {
		let rkey = Rkey::parse(HomeRecord::RKEY)?;
		if let Some(existing) =
			public.get_record(&HomeRecord::COLLECTION, &rkey).await?
			&& HomeRecord::accept(EntityBodyRecord::try_from(
				existing.value.clone(),
			)?)?
			.body()
			.get::<Home>()?
			.package == *package
		{
			writes.unchanged.push(existing.strong_ref());
			return Ok(());
		}
		let home = HomeRecord::accept(
			EntityBodyRecord::new(HomeRecord::COLLECTION)
				.with(&Provenance::new([]))?
				.with(&Home {
					package: package.clone(),
				})?,
		)?;
		public
			.converge_copy(
				public,
				&HomeRecord::COLLECTION,
				Rkeyed::new(rkey, AtprotoValue::from(home.into_body())),
				writes,
			)
			.await?;
		Ok(())
	}
}

/// What one [`PackageReleaser::release`] or [`PackageReleaser::publish`]
/// wrote, for the package and each path dependency before it.
#[derive(Debug, Clone)]
pub struct ReleaseReport {
	/// The package and version released or published.
	pub package: Package,
	/// The release as it stands: in the space for a release, in the public
	/// repo for a publish.
	pub record: StrongRef,
	/// Each record written or left alone.
	pub writes: ConvergeReport,
	/// Each path dependency's own release or publish.
	pub dependencies: Vec<ReleaseReport>,
}

impl ReleaseReport {
	/// Whether this run and its dependencies' wrote nothing.
	pub fn is_noop(&self) -> bool {
		self.writes.is_noop() && self.dependencies.iter().all(Self::is_noop)
	}
}

/// Each dependency's lines, then `<package>@<version>` and one converge line
/// per record.
impl core::fmt::Display for ReleaseReport {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		for dependency in &self.dependencies {
			write!(f, "{dependency}")?;
		}
		writeln!(f, "{}@{}", self.package.name, self.package.version)?;
		write!(f, "{}", self.writes)
	}
}

#[cfg(test)]
mod test {
	use super::super::package_compiler::test::compiler;
	use super::super::package_compiler::test::snapshot;
	use super::super::package_compiler::test::source;
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	const ENTRY: &str = "todos/main.bsx";

	/// The todos package at `version`, its page reading `page`.
	fn files(version: &str, page: &str) -> [(String, String); 3] {
		[
			(
				ENTRY.into(),
				format!(
					r#"<Package name="todos" version="{version}"/>
<Instance document="pages/About.bsx"/>"#
				),
			),
			("todos/pages/About.bsx".into(), format!("<p>{page}</p>")),
			("todos/logo.png".into(), "png".into()),
		]
	}

	/// Write `files` over `source`.
	async fn write(source: &BlobStore, files: &[(String, String)]) {
		for (path, text) in files {
			source
				.insert(&FwdPath::new(path).unwrap(), text.clone())
				.await
				.unwrap();
		}
	}

	async fn todos(version: &str) -> BlobStore {
		let source = source(&[]).await;
		write(&source, &files(version, "about")).await;
		source
	}

	/// Compile then release `source`'s todos package.
	async fn release(source: &BlobStore) -> Result<ReleaseReport> {
		let compiled = compiler(source)
			.compile(source, &fwd_path!(ENTRY))
			.await?;
		releaser(source).release(&compiled.package_record).await
	}

	fn releaser(source: &BlobStore) -> PackageReleaser {
		PackageReleaser::new(PackageCompiler::local_pds(source))
	}

	fn public(source: &BlobStore) -> Pds {
		PackageCompiler::local_pds(source).public().unwrap()
	}

	fn space(source: &BlobStore, name: &str) -> Pds {
		PackageCompiler::local_pds(source)
			.space(SpaceRef::package(name).unwrap())
			.unwrap()
	}

	/// The version `latest` holds in `repo`, `None` when there is none.
	async fn latest(repo: &Pds) -> Option<String> {
		repo.get::<ReleaseRecord>(&PackageReleaser::latest_rkey(
			&PackageName::parse("todos").unwrap(),
		))
		.await
		.unwrap()
		.map(|entry| {
			entry
				.value
				.body()
				.get::<Package>()
				.unwrap()
				.version
				.to_string()
		})
	}

	fn rkeys(refs: &[StrongRef]) -> Vec<String> {
		refs.iter()
			.map(|strong_ref| strong_ref.uri.rkey().to_string())
			.collect()
	}

	/// A release freezes the package record's body, and a second run
	/// changes nothing.
	#[beet_core::test]
	async fn releases_twice_unchanged() {
		let source = todos("1.2.0").await;
		let first = release(&source).await.unwrap();
		rkeys(&first.writes.created)
			.xpect_eq(vec!["todos:1.2.0", "todos:latest"]);
		let frozen = space(&source, "todos")
			.get::<ReleaseRecord>(first.record.uri.rkey())
			.await
			.unwrap()
			.unwrap()
			.value;
		let package = space(&source, "todos")
			.get::<PackageRecord>(&Rkey::parse("todos:main").unwrap())
			.await
			.unwrap()
			.unwrap()
			.value;
		// the same components, collection and identity apart
		frozen
			.body()
			.clone()
			.with_collection(PackageRecord::COLLECTION)
			.xpect_eq(package.body().clone());
		let before = snapshot(&source).await;
		let second = release(&source).await.unwrap();
		second.is_noop().xpect_true();
		second.record.xpect_eq(first.record);
		snapshot(&source).await.xpect_eq(before);
	}

	/// A version is never rewritten: changed content at a released version
	/// is refused naming the version and both cids.
	#[beet_core::test]
	async fn refuses_a_rewritten_version() {
		let source = todos("1.2.0").await;
		let first = release(&source).await.unwrap();
		write(&source, &files("1.2.0", "changed")).await;
		let err = release(&source).await.unwrap_err().to_string();
		err.clone()
			.xpect_contains("todos@1.2.0")
			.xpect_contains(first.record.cid.as_str())
			.xpect_contains("new version");
		// the cid the rewrite would have written beside the one standing
		err.matches("bafyrei").count().xpect_eq(2);
		// the release stands as it was
		space(&source, "todos")
			.get_record(&ReleaseRecord::COLLECTION, first.record.uri.rkey())
			.await
			.unwrap()
			.unwrap()
			.cid
			.xpect_eq(first.record.cid);
	}

	/// `latest` follows the newest plain version: a prerelease and an older
	/// version leave it alone.
	#[beet_core::test]
	async fn latest_is_the_newest_plain_version() {
		let source = todos("1.0.0").await;
		release(&source).await.unwrap();
		let space = space(&source, "todos");
		latest(&space).await.xpect_eq(Some("1.0.0".into()));
		write(&source, &files("2.0.0-alpha.1", "alpha")).await;
		release(&source)
			.await
			.unwrap()
			.writes
			.created
			.len()
			.xpect_eq(1);
		latest(&space).await.xpect_eq(Some("1.0.0".into()));
		write(&source, &files("2.0.0", "two")).await;
		release(&source).await.unwrap();
		latest(&space).await.xpect_eq(Some("2.0.0".into()));
		write(&source, &files("1.1.0", "patch")).await;
		release(&source).await.unwrap();
		latest(&space).await.xpect_eq(Some("2.0.0".into()));
		// latest is a whole copy, the release's cid at another rkey
		space
			.get_record(
				&ReleaseRecord::COLLECTION,
				&PackageReleaser::latest_rkey(
					&PackageName::parse("todos").unwrap(),
				),
			)
			.await
			.unwrap()
			.unwrap()
			.cid
			.xpect_eq(
				space
					.get_record(
						&ReleaseRecord::COLLECTION,
						&Rkey::parse("todos:2.0.0").unwrap(),
					)
					.await
					.unwrap()
					.unwrap()
					.cid,
			);
	}

	/// A release is private until published: nothing reaches the public
	/// repo.
	#[beet_core::test]
	async fn a_release_stays_private() {
		let source = todos("1.0.0").await;
		release(&source).await.unwrap();
		for collection in [ReleaseRecord::COLLECTION, SceneRecord::COLLECTION] {
			public(&source)
				.list_records(&collection)
				.await
				.unwrap()
				.xpect_empty();
		}
		source
			.list()
			.await
			.unwrap()
			.into_iter()
			.filter(|path| {
				path.as_str().starts_with(".beet/pds/blobs")
					|| path.as_str().starts_with(".beet/pds/records")
			})
			.count()
			.xpect_eq(0);
	}

	/// A publish copies the release, its scenes and every blob byte for
	/// byte, and a second publish is unchanged throughout.
	#[beet_core::test]
	async fn publishes_byte_for_byte() {
		let source = todos("1.0.0").await;
		let released = release(&source).await.unwrap();
		let published = releaser(&source)
			.publish(&released.record, false)
			.await
			.unwrap();
		published
			.record
			.cid
			.clone()
			.xpect_eq(released.record.cid.clone());
		published.record.uri.scope().xpect_eq(RepoScope::Public);
		// two scenes, the release and latest
		published.writes.created.len().xpect_eq(4);
		let public = public(&source);
		let release = public
			.get::<ReleaseRecord>(released.record.uri.rkey())
			.await
			.unwrap()
			.unwrap()
			.value;
		for blob in release.body().get::<Files>().unwrap().0.values() {
			public.get_blob(&blob.cid).await.unwrap().xpect_some();
		}
		for scene in release.body().get::<Scenes>().unwrap().0.values() {
			let copy = public
				.get_record(&SceneRecord::COLLECTION, scene.record.uri.rkey())
				.await
				.unwrap()
				.unwrap();
			copy.cid.clone().xpect_eq(scene.record.cid.clone());
			let body = copy.into_typed::<SceneRecord>().unwrap().value;
			public
				.get_blob(&body.body().get::<BlobRef>().unwrap().cid)
				.await
				.unwrap()
				.xpect_some();
		}
		latest(&public).await.xpect_eq(Some("1.0.0".into()));
		let before = snapshot(&source).await;
		let again = releaser(&source)
			.publish(&released.record, false)
			.await
			.unwrap();
		again.is_noop().xpect_true();
		again.writes.unchanged.len().xpect_eq(4);
		snapshot(&source).await.xpect_eq(before);
	}

	/// A public copy deleted by hand is copied again by the next publish,
	/// and only it.
	#[beet_core::test]
	async fn repairs_a_missing_copy() {
		let source = todos("1.0.0").await;
		let released = release(&source).await.unwrap();
		releaser(&source)
			.publish(&released.record, false)
			.await
			.unwrap();
		public(&source)
			.delete_record(
				&ReleaseRecord::COLLECTION,
				released.record.uri.rkey(),
			)
			.await
			.unwrap();
		let repaired = releaser(&source)
			.publish(&released.record, false)
			.await
			.unwrap();
		rkeys(&repaired.writes.created).xpect_eq(vec!["todos:1.0.0"]);
		repaired.record.cid.xpect_eq(released.record.cid);
	}

	/// A dry publish reports what it would write and writes nothing.
	#[beet_core::test]
	async fn a_dry_publish_writes_nothing() {
		let source = todos("1.0.0").await;
		let released = release(&source).await.unwrap();
		let before = snapshot(&source).await;
		let dry =
			PackageReleaser::new(PackageCompiler::local_pds(&source).dry())
				.publish(&released.record, true)
				.await
				.unwrap();
		dry.to_string().xpect_contains("would create");
		dry.writes.created.len().xpect_eq(5);
		snapshot(&source).await.xpect_eq(before);
	}

	/// The home record is written by the flag alone, and one naming another
	/// package is replaced only by it.
	#[beet_core::test]
	async fn writes_the_home_record_when_asked() {
		let source = todos("1.0.0").await;
		let released = release(&source).await.unwrap();
		let home = async || {
			public(&source)
				.get::<HomeRecord>(&Rkey::parse(HomeRecord::RKEY).unwrap())
				.await
				.unwrap()
				.map(|entry| {
					entry
						.value
						.body()
						.get::<Home>()
						.unwrap()
						.package
						.to_string()
				})
		};
		releaser(&source)
			.publish(&released.record, false)
			.await
			.unwrap();
		home().await.xpect_none();
		// another package's home stands without the flag
		let other = HomeRecord::accept(
			EntityBodyRecord::new(HomeRecord::COLLECTION)
				.with(&Provenance::default())
				.unwrap()
				.with(&Home {
					package: PackageName::parse("site").unwrap(),
				})
				.unwrap(),
		)
		.unwrap();
		public(&source).put(&other.rkeyed()).await.unwrap();
		releaser(&source)
			.publish(&released.record, false)
			.await
			.unwrap();
		home().await.xpect_eq(Some("site".into()));
		releaser(&source)
			.publish(&released.record, true)
			.await
			.unwrap();
		home().await.xpect_eq(Some("todos".into()));
		releaser(&source)
			.publish(&released.record, true)
			.await
			.unwrap()
			.is_noop()
			.xpect_true();
	}

	/// A path dependency is released first and pinned to that release, and
	/// published first.
	#[beet_core::test]
	async fn releases_a_path_dependency_first() {
		let source = source(&[
			(
				ENTRY,
				r#"<Package name="todos" version="1.0.0"><Dependency path="../widgets"/></Package>
<Instance document="widgets/Card.bsx"/>"#,
			),
			("widgets/main.bsx", r#"<Package name="widgets" version="0.2.0"/>"#),
			("widgets/Card.bsx", "<p>card</p>"),
		])
		.await;
		let released = release(&source).await.unwrap();
		released.dependencies.len().xpect_eq(1);
		let dependency = &released.dependencies[0];
		dependency
			.record
			.uri
			.to_string()
			.xpect_contains("/org.beet.release/widgets:0.2.0");
		let pinned = space(&source, "todos")
			.get::<ReleaseRecord>(released.record.uri.rkey())
			.await
			.unwrap()
			.unwrap()
			.value
			.body()
			.get::<Dependencies>()
			.unwrap()
			.0[&PackageName::parse("widgets").unwrap()]
			.resolved
			.clone()
			.unwrap();
		pinned.record.xpect_eq(dependency.record.clone());
		pinned.version.to_string().xpect_eq("0.2.0");
		let published = releaser(&source)
			.publish(&released.record, false)
			.await
			.unwrap();
		published.dependencies[0]
			.record
			.cid
			.clone()
			.xpect_eq(dependency.record.cid.clone());
		public(&source)
			.get::<ReleaseRecord>(&Rkey::parse("widgets:0.2.0").unwrap())
			.await
			.unwrap()
			.xpect_some();
	}
}
