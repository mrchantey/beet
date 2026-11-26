//! [`PackageCompiler`]: a branch compiled into scenes and one package record,
//! in the package's space.
use super::*;
use crate::prelude::*;
use alloc::collections::BTreeMap;
use beet_core::prelude::*;
use beet_net::prelude::*;
use bytes::Bytes;

/// Compiles one branch of a package into the package's space: every document
/// into a scene record keyed by its content, then a branch record of its files
/// and a package record pointing at the scenes, both at `<package>:<branch>`.
///
/// The compile runs in two stages, as a build tool's does. Each document is
/// lowered by its parser, every [`DocPath`] it holds is resolved package
/// absolute with its extension, and its scene meta is hoisted in a compile
/// world to which no plugin was added ([`SceneMeta`]), so a scene is a
/// function of its document alone. Then package meta sees them all: every
/// use's props are checked against its target's [`Props`], an instance cycle
/// is refused, one [`LoadSet`] is computed per document, and every
/// [`PackageMeta`] runs.
///
/// Nothing is rewritten that did not change: a scene is keyed by its
/// content, so an unchanged document keeps its record, and the branch and
/// package records are written only when their content moved, each keeping
/// the time it was first written. Compiling twice writes nothing the second
/// time.
///
/// A path dependency within the same home directory compiles first into its
/// own space, and its documents are named through its prefix.
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// # use beet_scene::prelude::*;
/// # async_ext::block_on(async {
/// let home = BlobStore::temp();
/// home.insert(&RelPath::new("todos/main.json"), r#"{"title":"todos"}"#).await?;
/// let world = ScenePlugin.into_world();
/// let repo = PackageCompiler::local_repo(&home);
/// let report = PackageCompiler::new(DocParsers::of(&world), repo)
/// 	.compile(&home, &RelPath::new("todos/main.json"))
/// 	.await?;
/// report.package.name.as_str().xpect_eq("todos");
/// report.package_record.uri.to_string().xpect_contains("/org.beet.package/todos:main");
/// # Ok::<_, BevyError>(())
/// # })
/// # .unwrap();
/// ```
#[derive(Clone)]
pub struct PackageCompiler {
	parsers: DocParsers,
	repo: Pds,
	branch: BranchName,
}

impl PackageCompiler {
	/// The did a home directory's repo answers for before any account syncs
	/// it, its own repo in each space laid out under `self` all the same.
	pub const LOCAL_DID: &'static str = "did:web:localhost";
	/// Where a home directory keeps its compiled form.
	pub const REPO_DIR: &'static str = ".beet/repo";
	/// The stem an entry document is named by, ie `main.bsx`, which is how a
	/// dependency's and a sub package's entry are found.
	pub const ENTRY_STEM: &'static str = "main";
	/// What a package's file listing never holds: the compiled form, git's
	/// own and a cargo build.
	const EXCLUDED: &'static [&'static str] = &[".beet", ".git", "target"];

	/// A compiler reading documents through `parsers`, whose types are the
	/// compiling binary's, and writing into `repo`'s account, each package
	/// into its own space.
	pub fn new(parsers: DocParsers, repo: Pds) -> Self {
		Self {
			parsers,
			repo,
			branch: BranchName::MAIN,
		}
	}

	/// Compile `branch` rather than `main`.
	pub fn with_branch(mut self, branch: BranchName) -> Self {
		self.branch = branch;
		self
	}

	/// The repo of the home directory `home`: an [`EmulatorPds`] under its
	/// [`REPO_DIR`](Self::REPO_DIR), the public repo at the top and each
	/// package's space beneath.
	pub fn local_repo(home: &BlobStore) -> Pds {
		Pds::new(EmulatorPds::new(
			Did::parse(Self::LOCAL_DID).expect("a valid did"),
			home.with_subdir(RelPath::new(Self::REPO_DIR)),
		))
	}

	/// Compile the package whose entry is `entry` in the home directory
	/// `home`: the entry's directory is the package's root.
	pub async fn compile(
		&self,
		home: &BlobStore,
		entry: &RelPath,
	) -> Result<CompileReport> {
		self.compile_package(home.clone(), entry.clone(), Vec::new())
			.await
			.map(|package| package.report)
	}

	/// The entry `path` of `home` names: the document itself, or the one entry
	/// of the directory it names ([`ENTRY_STEM`](Self::ENTRY_STEM)).
	pub async fn entry_of(
		&self,
		home: &BlobStore,
		path: &RelPath,
	) -> Result<RelPath> {
		match PackageSource::is_entry_like(path, &self.parsers)
			&& home.exists(path).await?
		{
			true => path.clone().xok(),
			false => Self::find_entry(home, path, &self.parsers).await,
		}
	}

	/// Whether the package record of `branch` no longer matches the branch
	/// record beside it, or is missing, read from the package's space.
	pub async fn is_stale(&self, branch: &Branch) -> Result<bool> {
		let space = self
			.repo
			.space(SpaceRef::package(branch.package.as_str())?)?;
		let rkey = branch.rkey();
		let (Some(branch_record), Some(package)) = (
			space.get_record(&BranchRecord::COLLECTION, &rkey).await?,
			space.get::<PackageRecord>(&rkey).await?,
		) else {
			return true.xok();
		};
		package
			.value
			.body()
			.get::<SourceBranch>()?
			.is_stale(&branch_record.cid)
			.xok()
	}

	/// Compile one package, its path dependencies first; `visiting` holds the
	/// entries of the packages depending on it, refusing a cycle.
	fn compile_package(
		&self,
		home: BlobStore,
		entry: RelPath,
		mut visiting: Vec<RelPath>,
	) -> LifetimeSendBoxedFuture<'_, Result<CompiledPackage>> {
		Box::pin(async move {
			if visiting.contains(&entry) {
				visiting.push(entry);
				bevybail!(
					"a path dependency cycle: {}",
					visiting
						.iter()
						.map(|entry| format!("`{entry}`"))
						.collect::<Vec<_>>()
						.join(" depends on ")
				);
			}
			visiting.push(entry.clone());
			let dir = entry.parent().unwrap_or_default();
			let store = match dir.is_empty() {
				true => home.clone(),
				false => home.with_subdir(dir.clone()),
			};
			let entry_path =
				entry.strip_prefix(&dir).unwrap_or_else(|| entry.clone());
			let mut source = PackageSource::read(&store, &self.parsers).await?;
			let entry_bytes = source.media(&entry_path).ok_or_else(|| {
				bevyhow!("the entry `{entry}` is no document of its package")
			})?;

			// the entry's declarations, read before anything resolves
			let declarations = CompiledDocument::declarations(
				&self.parsers,
				&entry_path,
				&entry_bytes,
			)?;
			let package = match declarations.try_get::<Package>()? {
				Some(package) => package,
				None => Package::implicit(
					dir.file_name()
						.map(str::to_string)
						.or_else(|| {
							home.base_dir().and_then(|dir| {
								dir.file_name().map(str::to_string)
							})
						})
						.unwrap_or_default()
						.as_str(),
				),
			};
			source.drop_sub_packages(&self.parsers, &entry_path)?;

			// path dependencies, each into its own space
			let mut namespace = Namespace {
				files: source.files.clone(),
				..default()
			};
			let mut scenes = SceneSet {
				records: default(),
				files: source.files.clone(),
			};
			let mut dependencies =
				declarations.try_get::<Dependencies>()?.unwrap_or_default();
			let mut dependency_reports = Vec::new();
			for (key, dependency) in dependencies.0.iter_mut() {
				if source.files.0.keys().any(|path| {
					path.as_str().split('/').next() == Some(key.as_str())
						&& path.as_str().contains('/')
				}) {
					bevybail!(
						"the dependency `{key}` collides with the directory \
						 `{key}/` of `{}`: both claim the first segment of a \
						 path, so alias the dependency",
						package.name
					);
				}
				let Some(path) = &dependency.path else {
					warn!(
						"`{}` depends on `{key}` by address alone, which this \
						 compile does not resolve: give it a `path`",
						package.name
					);
					continue;
				};
				let joined = dir.as_smol_path().join(path.as_str());
				if joined.as_str().split('/').next() == Some("..") {
					bevybail!(
						"the dependency `{key}` at `{path}` is outside the home \
						 directory: a path dependency is a package beside this \
						 one"
					);
				}
				let dependency_dir = RelPath::new(joined.as_str());
				let dependency_entry =
					Self::find_entry(&home, &dependency_dir, &self.parsers)
						.await?;
				let compiled = self
					.compile_package(
						home.clone(),
						dependency_entry,
						visiting.clone(),
					)
					.await?;
				dependency.resolved = Some(ResolvedDependency {
					did: self.repo.did().clone(),
					version: compiled.report.package.version.clone(),
					record: compiled.report.package_record.clone(),
				});
				let prefixed = |path: &RelPath| {
					RelPath::new(key.as_str()).join(path.as_str())
				};
				for (path, blob) in &compiled.files.0 {
					scenes.files.0.insert(prefixed(path), blob.clone());
				}
				for (path, record) in compiled.scenes {
					scenes.records.insert(prefixed(&path), record);
				}
				namespace.dependencies.insert(key.clone(), compiled.files);
				dependency_reports.push(compiled.report);
			}

			// every document into a scene, then what needs them all
			let documents = source
				.documents()
				.into_iter()
				.map(|(path, bytes)| {
					CompiledDocument::compile(
						&self.parsers,
						&namespace,
						&path,
						&bytes,
					)
				})
				.collect::<Result<Vec<_>>>()?;
			let document_paths = documents
				.iter()
				.map(|document| document.path.clone())
				.collect::<Vec<_>>();
			for document in &documents {
				scenes
					.records
					.insert(document.path.clone(), document.record.clone());
			}
			scenes.check_props(&documents)?;
			scenes.check_cycles()?;
			let load_sets = scenes.load_sets(&document_paths)?;
			let package_meta = {
				let types = self.parsers.types().clone();
				let registry = types.read();
				let mut world = World::new();
				world.insert_resource(types.clone());
				let meta = world.spawn_empty().id();
				scenes.apply_package_meta(
					&mut world,
					&registry,
					&document_paths,
					meta,
				)?;
				CompiledDocument::record_of(
					&world,
					&registry,
					meta,
					PackageRecord::COLLECTION,
				)?
			};
			let entry_record = documents
				.iter()
				.find(|document| document.path == entry_path)
				.map(|document| document.record.clone())
				.expect("the entry is a document");

			// the writes, each only where its content moved
			let space =
				self.repo.space(SpaceRef::package(package.name.as_str())?)?;
			let mut writer = SpaceWriter::new(space);
			let branch = Branch::new(package.name.clone(), self.branch.clone());
			let existing_files = writer
				.existing::<BranchRecord>(&branch.rkey())
				.await?
				.and_then(|record| {
					record.body().try_get::<Files>().ok().flatten()
				})
				.unwrap_or_default();
			for (path, blob) in &source.files.0 {
				if existing_files.0.get(path) != Some(blob) {
					writer
						.upload(
							source.bytes[path].clone(),
							blob.mime_type.clone(),
						)
						.await?;
				}
			}
			let mut scene_refs = Scenes::default();
			for document in &documents {
				let record = writer.write_scene(document).await?;
				scene_refs.0.insert(document.path.clone(), SceneRef {
					record,
					heads: default(),
				});
			}
			let mut branch_body =
				EntityBodyRecord::new(BranchRecord::COLLECTION)
					.with(&branch)?
					.with(&source.files)?;
			Self::copy::<Package>(
				&declarations,
				&mut branch_body,
				Some(&package),
			)?;
			Self::copy::<Exports>(&declarations, &mut branch_body, None)?;
			Self::copy::<Dependencies>(&declarations, &mut branch_body, None)?;
			Self::copy::<ExportRuntimes>(
				&declarations,
				&mut branch_body,
				None,
			)?;
			let branch_ref = writer.write::<BranchRecord>(branch_body).await?;

			let mut package_body = package_meta
				.with(&branch)?
				.with(&SourceBranch(branch_ref.clone()))?
				.with(&package)?
				.with(&scene_refs)?
				.with(&source.files)?
				.with(&load_sets)?;
			if !dependencies.0.is_empty() {
				package_body.insert(&dependencies)?;
			}
			Self::copy::<Exports>(&declarations, &mut package_body, None)?;
			Self::copy::<ExportRuntimes>(
				&declarations,
				&mut package_body,
				None,
			)?;
			Self::copy::<RequireCfg>(&entry_record, &mut package_body, None)?;
			let package_ref =
				writer.write::<PackageRecord>(package_body).await?;

			CompiledPackage {
				files: source.files,
				scenes: documents
					.into_iter()
					.map(|document| (document.path, document.record))
					.collect(),
				report: writer.report(
					package,
					self.branch.clone(),
					branch_ref,
					package_ref,
					dependency_reports,
				),
			}
			.xok()
		})
	}

	/// Copy `T` from `from` into `into` where it holds one, else `fallback`.
	fn copy<T: Def + Serialize + DeserializeOwned>(
		from: &EntityBodyRecord,
		into: &mut EntityBodyRecord,
		fallback: Option<&T>,
	) -> Result {
		match from.try_get::<T>()?.as_ref().or(fallback) {
			Some(value) => into.insert(value),
			None => Ok(()),
		}
	}

	/// The entry of the package in `dir` of `home`: its one document named
	/// [`ENTRY_STEM`](Self::ENTRY_STEM).
	async fn find_entry(
		home: &BlobStore,
		dir: &RelPath,
		parsers: &DocParsers,
	) -> Result<RelPath> {
		let listed = home.list_dir(dir).await?;
		let entries = listed
			.files
			.iter()
			.map(|file| dir.join(file.as_str()))
			.filter(|path| PackageSource::is_entry(path, parsers))
			.collect::<Vec<_>>();
		match entries.as_slice() {
			[entry] => entry.clone().xok(),
			[] => bevybail!(
				"`{dir}` holds no entry: a package's entry is its `{}` \
				 document, ie `{}.bsx`",
				Self::ENTRY_STEM,
				Self::ENTRY_STEM
			),
			many => bevybail!(
				"`{dir}` holds {} entries, {}: a package has one",
				many.len(),
				many.iter()
					.map(|path| format!("`{path}`"))
					.collect::<Vec<_>>()
					.join(" and ")
			),
		}
	}
}

/// One package compiled, what a package depending on it reads.
struct CompiledPackage {
	files: Files,
	scenes: BTreeMap<RelPath, EntityBodyRecord>,
	report: CompileReport,
}

/// A package's files as read from its directory: every path's blob reference
/// and bytes.
struct PackageSource {
	files: Files,
	bytes: BTreeMap<RelPath, Bytes>,
	/// Which of the files are documents, by their parser.
	documents: BTreeMap<RelPath, MediaType>,
}

impl PackageSource {
	/// Every file of the package `store` is rooted at, but what
	/// [`PackageCompiler`] excludes.
	async fn read(store: &BlobStore, parsers: &DocParsers) -> Result<Self> {
		let mut source = Self {
			files: default(),
			bytes: default(),
			documents: default(),
		};
		for path in store.list().await? {
			if path
				.as_str()
				.split('/')
				.next()
				.is_some_and(|first| PackageCompiler::EXCLUDED.contains(&first))
			{
				continue;
			}
			let media_type = path.media_type().unwrap_or(MediaType::Bytes);
			let bytes = store.get(&path).await?;
			if parsers.reads(&media_type) {
				source.documents.insert(path.clone(), media_type.clone());
			}
			source
				.files
				.0
				.insert(path.clone(), BlobRef::of(&bytes, media_type));
			source.bytes.insert(path, bytes);
		}
		source.xok()
	}

	/// Whether `path` names an entry: a document whose stem is
	/// [`PackageCompiler::ENTRY_STEM`].
	fn is_entry(path: &RelPath, parsers: &DocParsers) -> bool {
		path.file_stem() == Some(PackageCompiler::ENTRY_STEM)
			&& path
				.media_type()
				.is_some_and(|media_type| parsers.reads(&media_type))
	}

	/// Whether `path` names a document of a type `parsers` reads.
	fn is_entry_like(path: &RelPath, parsers: &DocParsers) -> bool {
		path.media_type()
			.is_some_and(|media_type| parsers.reads(&media_type))
	}

	/// The document at `path` as its bytes and media type.
	fn media(&self, path: &RelPath) -> Option<MediaBytes> {
		let media_type = self.documents.get(path)?;
		MediaBytes::new(media_type.clone(), self.bytes.get(path)?.clone())
			.xsome()
	}

	/// Every document, in path order.
	fn documents(&self) -> Vec<(RelPath, MediaBytes)> {
		self.documents
			.keys()
			.filter_map(|path| Some((path.clone(), self.media(path)?)))
			.collect()
	}

	/// Leave out every sub package: a directory below the root whose entry
	/// declares a [`Package`] of its own.
	fn drop_sub_packages(
		&mut self,
		parsers: &DocParsers,
		entry: &RelPath,
	) -> Result {
		let roots = self
			.documents
			.keys()
			.filter(|path| *path != entry && Self::is_entry(path, parsers))
			.cloned()
			.collect::<Vec<_>>();
		for root in roots {
			let Some(bytes) = self.media(&root) else {
				continue;
			};
			if CompiledDocument::declarations(parsers, &root, &bytes)?
				.contains(&Package::NSID)
			{
				let dir = root.parent().unwrap_or_default();
				let within = |path: &RelPath| path.strip_prefix(&dir).is_some();
				self.files.0.retain(|path, _| !within(path));
				self.bytes.retain(|path, _| !within(path));
				self.documents.retain(|path, _| !within(path));
			}
		}
		Ok(())
	}
}

#[cfg(all(test, feature = "bsx"))]
mod test {
	use crate::prelude::*;
	use alloc::collections::BTreeMap;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use bevy::ecs::lifecycle::HookContext;
	use bevy::ecs::world::DeferredWorld;
	use core::sync::atomic::AtomicBool;
	use core::sync::atomic::Ordering;

	/// A plugin's route, hoisted onto its scene and gathered into the package
	/// record's table.
	#[derive(
		Debug,
		Default,
		Clone,
		PartialEq,
		Component,
		Reflect,
		Def,
		Serialize,
		Deserialize,
	)]
	#[def(lexicon = "com.example.router")]
	#[reflect(
		Component,
		Default,
		Def,
		SceneMeta,
		PackageMeta,
		Serialize,
		Deserialize
	)]
	struct Route {
		path: SmolStr,
	}

	impl SceneMeta for Route {
		fn apply(cx: &mut SceneMetaContext) -> Result {
			let route = cx.get::<Self>()?.clone();
			cx.insert_meta(route);
			Ok(())
		}
	}

	impl PackageMeta for Route {
		fn apply(cx: &mut PackageMetaContext) -> Result {
			let route = cx.scene().get::<Self>()?;
			let document = cx.document.clone();
			cx.meta_mut::<RouteTable>().0.insert(route.path, document);
			Ok(())
		}
	}

	/// The package's routes, every page's.
	#[derive(
		Debug,
		Default,
		Clone,
		PartialEq,
		Component,
		Reflect,
		Def,
		Serialize,
		Deserialize,
	)]
	#[def(lexicon = "com.example.router")]
	#[reflect(Component, Default, Def, Serialize, Deserialize)]
	struct RouteTable(BTreeMap<SmolStr, RelPath>);

	static FIRED: AtomicBool = AtomicBool::new(false);

	/// Fires as it lands, which a compile must never let it do.
	#[derive(Default, Component, Reflect)]
	#[reflect(Component, Default)]
	#[component(on_add = fire)]
	struct Fires;

	fn fire(_world: DeferredWorld, _cx: HookContext) {
		FIRED.store(true, Ordering::SeqCst);
	}

	/// A world reading every format, with the test plugin's types.
	fn world() -> World {
		let world = ScenePlugin.into_world();
		{
			let mut registry = world.resource::<AppTypeRegistry>().write();
			registry.register::<RequireCfg>();
			registry.register::<Route>();
			registry.register::<RouteTable>();
			registry.register::<Fires>();
			registry.register::<CallOnReady>();
		}
		world
	}

	async fn home(files: &[(&str, &str)]) -> BlobStore {
		let home = BlobStore::temp();
		for (path, source) in files {
			home.insert(&RelPath::new(path), source.to_string())
				.await
				.unwrap();
		}
		home
	}

	fn compiler(home: &BlobStore) -> PackageCompiler {
		PackageCompiler::new(
			DocParsers::of(&world()),
			PackageCompiler::local_repo(home),
		)
	}

	async fn compile(home: &BlobStore) -> Result<CompileReport> {
		compiler(home)
			.compile(home, &RelPath::new("todos/main.bsx"))
			.await
	}

	/// Every file of `store` and its bytes, in path order.
	async fn snapshot(store: &BlobStore) -> Vec<(RelPath, Vec<u8>)> {
		let mut files = store
			.get_all()
			.await
			.unwrap()
			.into_iter()
			.map(|(path, bytes)| (path, bytes.to_vec()))
			.collect::<Vec<_>>();
		files.sort();
		files
	}

	fn space(home: &BlobStore, name: &str) -> Pds {
		PackageCompiler::local_repo(home)
			.space(SpaceRef::package(name).unwrap())
			.unwrap()
	}

	async fn package_record(home: &BlobStore, rkey: &str) -> PackageRecord {
		space(home, "todos")
			.get::<PackageRecord>(&Rkey::parse(rkey).unwrap())
			.await
			.unwrap()
			.unwrap()
			.value
	}

	async fn scene(
		home: &BlobStore,
		report: &CompileReport,
		path: &str,
	) -> SceneRecord {
		let package =
			package_record(home, report.package_record.uri.rkey().as_str())
				.await;
		let scenes = package.body().get::<Scenes>().unwrap();
		let scene_ref = &scenes.0[&RelPath::new(path)];
		space(home, "todos")
			.get::<SceneRecord>(scene_ref.record.uri.rkey())
			.await
			.unwrap()
			.unwrap()
			.value
	}

	/// The scene body a scene record keys, read back from its blob.
	async fn body(home: &BlobStore, scene: &SceneRecord) -> SceneBody {
		let blob = scene.body().get::<BlobRef>().unwrap();
		let bytes = home
			.get(&RelPath::new(format!(
				".beet/repo/spaces/org.beet.space.package/todos/self/blobs/{}",
				blob.cid
			)))
			.await
			.unwrap();
		Cid::raw(&bytes).xpect_eq(blob.cid.clone());
		MediaType::Postcard
			.deserialize::<SceneBody>(&bytes)
			.unwrap()
	}

	const MAIN: &str = r#"<Package name="todos" version="0.1.0">
	<Export key="." path="widgets/Card.bsx"/>
</Package>
<main {Route{path:"/"}}>
	<Instance document="./widgets/Card" props={{title:"One"}}/>
	<Instance document="pages/About.bsx" {LoadRoot}/>
</main>"#;
	const CARD: &str = r#"<Prop name="title" kind="string"/>
<Prop name="count" kind="u32" default={3}/>
<RequireCfg cfg="feature:web"/>
<section><h2>{@prop:title}</h2></section>"#;

	async fn todos() -> BlobStore {
		home(&[
			("todos/main.bsx", MAIN),
			("todos/widgets/Card.bsx", CARD),
			(
				"todos/pages/About.bsx",
				r#"<p {Route{path:"/about"}}>about</p>"#,
			),
			("todos/logo.png", "png"),
		])
		.await
	}

	#[beet_core::test]
	async fn compiles_twice_byte_identical() {
		let home = todos().await;
		let first = compile(&home).await.unwrap();
		// three scenes, the branch record and the package record
		first.written.len().xpect_eq(5);
		// four files and three scene bodies
		first.uploaded.len().xpect_eq(7);
		first
			.package_record
			.uri
			.to_string()
			.xpect_contains("/space/org.beet.space.package/todos/did:web:localhost/org.beet.package/todos:main");
		let before = snapshot(&home).await;
		let second = compile(&home).await.unwrap();
		second.wrote_nothing().xpect_true();
		second.package_record.xpect_eq(first.package_record);
		snapshot(&home).await.xpect_eq(before);
	}

	#[beet_core::test]
	async fn writes_the_package_record() {
		let home = todos().await;
		let report = compile(&home).await.unwrap();
		let package = package_record(&home, "todos:main").await;
		let body = package.body();
		body.get::<Package>()
			.unwrap()
			.version
			.as_str()
			.xpect_eq("0.1.0");
		body.get::<Exports>().unwrap().0[&SmolStr::new(".")]
			.xpect_eq(RelPath::new("widgets/Card.bsx"));
		body.get::<Files>().unwrap().0.len().xpect_eq(4);
		body.get::<Scenes>()
			.unwrap()
			.0
			.keys()
			.map(|path| path.to_string())
			.collect::<Vec<_>>()
			.xpect_eq(vec!["main.bsx", "pages/About.bsx", "widgets/Card.bsx"]);
		body.get::<SourceBranch>()
			.unwrap()
			.0
			.xpect_eq(report.branch_record.clone());
		// a plugin's package meta, gathered from every scene
		body.get::<RouteTable>()
			.unwrap()
			.0
			.xpect_eq(BTreeMap::from([
				("/".into(), RelPath::new("main.bsx")),
				("/about".into(), RelPath::new("pages/About.bsx")),
			]));
		// the entry's set stops at the lazy page, and carries the card's
		// requirement
		let card = scene(&home, &report, "widgets/Card.bsx").await;
		let main = scene(&home, &report, "main.bsx").await;
		let load_sets = body.get::<LoadSets>().unwrap();
		let set = &load_sets.0[&RelPath::new("main.bsx")];
		set.scenes.xpect_eq(vec![
			main.body().get::<BlobRef>().unwrap().cid,
			card.body().get::<BlobRef>().unwrap().cid,
		]);
		set.requires.clone().xpect_eq(Some("feature:web".into()));
		load_sets.0[&RelPath::new("pages/About.bsx")]
			.requires
			.clone()
			.xpect_none();
	}

	#[beet_core::test]
	async fn hoists_scene_meta() {
		let home = todos().await;
		let report = compile(&home).await.unwrap();
		let main = scene(&home, &report, "main.bsx").await;
		main.body()
			.get::<Route>()
			.unwrap()
			.path
			.as_str()
			.xpect_eq("/");
		main.body()
			.get::<Package>()
			.unwrap()
			.name
			.as_str()
			.xpect_eq("todos");
		let imports = main.body().get::<Imports>().unwrap();
		imports.0[&RelPath::new("widgets/Card.bsx")].xpect_eq(ImportEdge {
			kind: ImportKind::Eager,
			instance: true,
		});
		imports.0[&RelPath::new("pages/About.bsx")]
			.kind
			.xpect_eq(ImportKind::Lazy);
		let card = scene(&home, &report, "widgets/Card.bsx").await;
		let props = card.body().get::<Props>().unwrap();
		props.0["title"].kind.as_str().xpect_eq("string");
		props.0["count"]
			.default
			.clone()
			.xpect_eq(Some(Value::Uint(3)));
		card.body()
			.get::<RequireCfg>()
			.unwrap()
			.cfg
			.as_str()
			.xpect_eq("feature:web");
	}

	/// A reference is written package absolute with its extension in the
	/// scene body, so a record only ever holds a full path.
	#[beet_core::test]
	async fn resolves_references_in_the_body() {
		let home = todos().await;
		let report = compile(&home).await.unwrap();
		let main = scene(&home, &report, "main.bsx").await;
		let documents = body(&home, &main)
			.await
			.entities
			.values()
			.filter_map(|entity| {
				entity
					.iter()
					.find(|(name, _)| name.ends_with("Instance"))
					.and_then(|(_, instance)| {
						instance.as_map().ok()?.get("document").ok().cloned()
					})
					.map(|document| document.as_str().unwrap().to_string())
			})
			.collect::<Vec<_>>();
		documents.xpect_eq(vec!["widgets/Card.bsx", "pages/About.bsx"]);
	}

	#[beet_core::test]
	async fn one_changed_document_changes_one_scene() {
		let home = todos().await;
		let first = compile(&home).await.unwrap();
		home.insert(
			&RelPath::new("todos/widgets/Card.bsx"),
			format!("{CARD}<footer/>"),
		)
		.await
		.unwrap();
		let second = compile(&home).await.unwrap();
		// the card's scene, the branch record and the package record
		second.written.len().xpect_eq(3);
		// the card's source and its new body
		second.uploaded.len().xpect_eq(2);
		second
			.written
			.iter()
			.filter(|written| {
				written.uri.collection() == &SceneRecord::COLLECTION
			})
			.count()
			.xpect_eq(1);
		second
			.package_record
			.cid
			.xpect_not_eq(first.package_record.cid);
	}

	#[beet_core::test]
	async fn two_branches_two_package_records() {
		let home = todos().await;
		let main = compile(&home).await.unwrap();
		let dev = compiler(&home)
			.with_branch(BranchName::parse("dev").unwrap())
			.compile(&home, &RelPath::new("todos/main.bsx"))
			.await
			.unwrap();
		dev.package_record.uri.rkey().as_str().xpect_eq("todos:dev");
		// one version, two branches, no collision, the scenes shared
		package_record(&home, "todos:main").await;
		package_record(&home, "todos:dev").await;
		dev.written
			.iter()
			.filter(|written| {
				written.uri.collection() == &SceneRecord::COLLECTION
			})
			.count()
			.xpect_eq(0);
		dev.package_record.xpect_not_eq(main.package_record);
	}

	#[beet_core::test]
	async fn stale_once_its_branch_moves() {
		let home = todos().await;
		compile(&home).await.unwrap();
		let branch =
			Branch::new(PackageName::parse("todos").unwrap(), BranchName::MAIN);
		let compiler = compiler(&home);
		compiler.is_stale(&branch).await.unwrap().xpect_false();
		// a runtime with no compiler writes the branch: its files move on
		let space = space(&home, "todos");
		let record = space
			.get::<BranchRecord>(&branch.rkey())
			.await
			.unwrap()
			.unwrap()
			.value;
		let mut files = record.body().get::<Files>().unwrap();
		files.0.remove(&RelPath::new("logo.png"));
		let mut body = record.into_body();
		body.insert(&files).unwrap();
		space
			.put(&BranchRecord::accept(body).unwrap().rkeyed())
			.await
			.unwrap();
		compiler.is_stale(&branch).await.unwrap().xpect_true();
		compile(&home).await.unwrap();
		compiler.is_stale(&branch).await.unwrap().xpect_false();
	}

	/// A space is its own root: the public repo holds nothing a compile
	/// wrote, its blobs included.
	#[beet_core::test]
	async fn writes_the_space_alone() {
		let home = todos().await;
		compile(&home).await.unwrap();
		let paths = snapshot(&home)
			.await
			.into_iter()
			.map(|(path, _)| path.to_string())
			.filter(|path| path.starts_with(".beet/repo/"))
			.collect::<Vec<_>>();
		paths.is_empty().xpect_false();
		for path in paths {
			path.xpect_starts_with(
				".beet/repo/spaces/org.beet.space.package/todos/self/",
			);
		}
	}

	/// The skeleton holds only what takes part in compilation, so no other
	/// component's hook runs: a `CallOnReady` never fires.
	#[beet_core::test]
	async fn runs_no_runtime_behaviour() {
		let home =
			home(&[("todos/main.bsx", r#"<div {CallOnReady}><Fires/></div>"#)])
				.await;
		compile(&home).await.unwrap();
		FIRED.load(Ordering::SeqCst).xpect_false();
		// the same document fires once it is built
		let mut world = world();
		BsxParser::spawn(&mut world, r#"<div {CallOnReady}><Fires/></div>"#)
			.unwrap();
		FIRED.load(Ordering::SeqCst).xpect_true();
	}

	/// Every child of `Package` hoists onto the entry's scene, the branch
	/// record and the package record.
	#[beet_core::test]
	async fn hoists_the_package_declaration() {
		let home = home(&[(
			"todos/main.bsx",
			r#"<Package name="todos" version="0.1.0">
	<Import name="." from="widgets"/>
	<Runtime name="web" platform={Browser{wasm:"web.wasm", js:"web.js"}}/>
	<Export key="." path="main.bsx"/>
</Package>"#,
		)])
		.await;
		compile(&home).await.unwrap();
		let package = package_record(&home, "todos:main").await;
		let body = package.body();
		body.get::<Package>()
			.unwrap()
			.imports
			.xpect_eq(vec![Import {
				name: ".".into(),
				from: PackageName::parse("widgets").unwrap(),
			}]);
		let runtimes = body.get::<ExportRuntimes>().unwrap();
		runtimes.0.len().xpect_eq(1);
		runtimes.0[0]
			.platform
			.triple()
			.xpect_eq("wasm32-unknown-unknown");
		let branch = space(&home, "todos")
			.get::<BranchRecord>(&Rkey::parse("todos:main").unwrap())
			.await
			.unwrap()
			.unwrap()
			.value;
		branch.body().get::<Exports>().unwrap().0.len().xpect_eq(1);
		branch
			.body()
			.get::<Package>()
			.unwrap()
			.imports
			.len()
			.xpect_eq(1);
	}

	#[beet_core::test]
	async fn refuses_a_declaration_outside_its_package() {
		compile_err(&[(
			"todos/main.bsx",
			r#"<Export key="." path="main.bsx"/>"#,
		)])
		.await
		.xpect_contains("`Export` is a child of `Package`");
	}

	#[beet_core::test]
	async fn compiles_an_implicit_package() {
		let home = home(&[("my-site/main.bsx", "<p>hi</p>")]).await;
		let report = compiler(&home)
			.compile(&home, &RelPath::new("my-site/main.bsx"))
			.await
			.unwrap();
		report.package.name.as_str().xpect_eq("my-site");
		report.package.version.xpect_eq(Version::IMPLICIT);
	}

	#[beet_core::test]
	async fn leaves_out_a_sub_package() {
		let home = home(&[
			("todos/main.bsx", "<p>hi</p>"),
			(
				"todos/inner/main.bsx",
				r#"<Package name="inner" version="0.1.0"/>"#,
			),
			("todos/inner/page.bsx", "<p>inner</p>"),
		])
		.await;
		let report = compile(&home).await.unwrap();
		package_record(&home, "todos:main")
			.await
			.body()
			.get::<Files>()
			.unwrap()
			.0
			.len()
			.xpect_eq(1);
		report.dependencies.xpect_empty();
	}

	#[beet_core::test]
	async fn compiles_a_path_dependency() {
		let home = home(&[
			(
				"todos/main.bsx",
				r#"<Package name="todos" version="0.1.0"><Dependency path="../widgets"/></Package>
<Instance document="widgets/Card.bsx" props={{title:"x"}}/>"#,
			),
			("widgets/main.bsx", r#"<Package name="widgets" version="0.2.0"/>"#),
			("widgets/Card.bsx", CARD),
		])
		.await;
		let report = compile(&home).await.unwrap();
		report.dependencies.len().xpect_eq(1);
		let dependency = &report.dependencies[0];
		dependency.package.name.as_str().xpect_eq("widgets");
		let package = package_record(&home, "todos:main").await;
		let resolved = package.body().get::<Dependencies>().unwrap().0
			[&PackageName::parse("widgets").unwrap()]
			.resolved
			.clone()
			.unwrap();
		resolved.version.as_str().xpect_eq("0.2.0");
		resolved.record.xpect_eq(dependency.package_record.clone());
		// the dependency's card is in the entry's set
		package.body().get::<LoadSets>().unwrap().0[&RelPath::new("main.bsx")]
			.scenes
			.len()
			.xpect_eq(2);
	}

	async fn compile_err(files: &[(&str, &str)]) -> String {
		compile(&home(files).await).await.unwrap_err().to_string()
	}

	#[beet_core::test]
	async fn refuses_an_ambiguous_reference() {
		compile_err(&[
			("todos/main.bsx", r#"<Instance document="./Card"/>"#),
			("todos/Card.bsx", "<p/>"),
			("todos/Card.md", "card"),
		])
		.await
		.xpect_contains(
			"`./Card` is ambiguous, naming `Card.bsx` and `Card.md`",
		)
		.xpect_contains("main.bsx:1:1");
	}

	#[beet_core::test]
	async fn refuses_a_missing_reference() {
		compile_err(&[("todos/main.bsx", r#"<Instance document="./Card"/>"#)])
			.await
			.xpect_contains("`./Card` names no file of the package");
	}

	#[beet_core::test]
	async fn refuses_a_prefix_collision() {
		compile_err(&[
			(
				"todos/main.bsx",
				r#"<Package name="todos" version="0.1.0"><Dependency path="../widgets"/></Package>"#,
			),
			("todos/widgets/Card.bsx", "<p/>"),
		])
		.await
		.xpect_contains("collides with the directory `widgets/`");
	}

	#[beet_core::test]
	async fn refuses_a_prop_of_another_kind() {
		compile_err(&[
			(
				"todos/main.bsx",
				r#"<Instance document="Card.bsx" props={{title:3}}/>"#,
			),
			("todos/Card.bsx", CARD),
		])
		.await
		.xpect_contains("`Card.bsx` takes `title` as `string`, not");
	}

	#[beet_core::test]
	async fn refuses_a_missing_prop() {
		compile_err(&[
			("todos/main.bsx", r#"<Instance document="Card.bsx"/>"#),
			("todos/Card.bsx", CARD),
		])
		.await
		.xpect_contains("`Card.bsx` requires the prop `title`");
	}

	#[beet_core::test]
	async fn refuses_an_unknown_prop() {
		compile_err(&[
			(
				"todos/main.bsx",
				r#"<Instance document="Card.bsx" props={{title:"a", colour:"red"}}/>"#,
			),
			("todos/Card.bsx", CARD),
		])
		.await
		.xpect_contains("takes no prop `colour`");
	}

	/// A document declaring no props has no interface to check, whatever a
	/// use passes; the runtime binds what it reads.
	#[beet_core::test]
	async fn leaves_an_undeclared_interface_unchecked() {
		compile(
			&home(&[
				(
					"todos/main.bsx",
					r#"<Instance document="Card.bsx" props={{title:3}}/>"#,
				),
				("todos/Card.bsx", "<h2>{@prop:title}</h2>"),
			])
			.await,
		)
		.await
		.unwrap();
	}

	#[beet_core::test]
	async fn refuses_an_instance_cycle() {
		compile_err(&[
			("todos/main.bsx", r#"<Instance document="A.bsx"/>"#),
			("todos/A.bsx", r#"<Instance document="B.bsx" {LoadRoot}/>"#),
			("todos/B.bsx", r#"<Instance document="A.bsx"/>"#),
		])
		.await
		.xpect_contains(
			"an instance cycle: `A.bsx` instantiates `B.bsx` instantiates `A.bsx`",
		);
	}
}
