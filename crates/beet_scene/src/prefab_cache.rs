//! [`PrefabCache`]: each document a package instantiates, loaded once.
use beet_core::prelude::*;
use beet_net::prelude::*;
use std::sync::Arc;

/// A document held to be instantiated: its body, which every use builds from.
///
/// Held as data rather than as entities, since what a document says runs where
/// it is used: a template expands per use, under the request rendering it, a
/// default is minted per use, and a component acts as it lands, a server
/// binding its port or a layout joining its middleware, which a held copy must
/// never do.
pub struct InstancePrefab {
	/// The document's path in its package, its extension written.
	path: RelPath,
	body: SceneBody,
	/// The props the document declares.
	props: Vec<Prop>,
	/// The documents it instantiates, package absolute.
	instances: Vec<RelPath>,
}

impl InstancePrefab {
	/// The document's path in its package.
	pub fn path(&self) -> &RelPath { &self.path }

	/// What one use builds.
	///
	/// ## Errors
	/// Errors when the body no longer reads against `world`'s registry.
	pub fn reflect(&self, world: &World) -> Result<DynamicTemplate> {
		self.body.reflect_at(world, &self.path)
	}

	/// The props the document declares.
	pub fn props(&self) -> &[Prop] { &self.props }
}

/// The documents loaded so far, by the package they are read from and their
/// path in it. A change to one in its store forgets it, so its next use reads
/// it again.
#[derive(Default, Clone, Resource)]
pub struct PrefabCache(HashMap<PrefabKey, CachedPrefab>);

/// A document by the store and scope of its package and its path there.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PrefabKey {
	root_key: SmolStr,
	subdir: RelPath,
	path: RelPath,
}

impl PrefabKey {
	fn new(package: &PackageStore, path: &RelPath) -> Self {
		Self {
			root_key: package.root_key(),
			subdir: package.subdir(),
			path: path.clone(),
		}
	}
}

#[derive(Clone)]
struct CachedPrefab {
	package: PackageStore,
	prefab: Arc<InstancePrefab>,
}

impl PrefabCache {
	/// The prefab at `path` of `package`, if it is loaded. A path whose
	/// extension was left off is answered once it has been loaded by it.
	pub fn get(
		&self,
		package: &PackageStore,
		path: &RelPath,
	) -> Option<Arc<InstancePrefab>> {
		self.0
			.get(&PrefabKey::new(package, path))
			.map(|cached| cached.prefab.clone())
	}

	/// The prefab at `path` of `package` with every document it instantiates,
	/// loading what is not loaded, so building it finds each at hand.
	///
	/// ## Errors
	/// Errors when a document is missing, ambiguous or does not parse, or when
	/// a document instantiates itself, however indirectly.
	pub async fn load(
		world: &AsyncWorld,
		package: &PackageStore,
		path: &RelPath,
	) -> Result<Arc<InstancePrefab>> {
		Self::load_under(world, package, path, Vec::new()).await
	}

	/// Load every document `template`, the document at `path`, instantiates.
	pub async fn load_instances(
		world: &AsyncWorld,
		package: &PackageStore,
		path: &RelPath,
		template: &DynamicTemplate,
	) -> Result {
		for document in template.instances() {
			Self::load_under(world, package, &document, vec![path.clone()])
				.await?;
		}
		Ok(())
	}

	/// [`load`](Self::load) a document reached through `above`, outermost
	/// first.
	async fn load_under(
		world: &AsyncWorld,
		package: &PackageStore,
		path: &RelPath,
		above: Vec<RelPath>,
	) -> Result<Arc<InstancePrefab>> {
		let loaded = {
			let (package, path) = (package.clone(), path.clone());
			world
				.with(move |world: &mut World| {
					world
						.get_resource::<Self>()
						.and_then(|cache| cache.get(&package, &path))
				})
				.await
		};
		let prefab = match loaded {
			Some(prefab) => prefab,
			None => Self::read(world, package, path).await?,
		};
		if above.contains(&prefab.path) {
			bevybail!(
				"`{}` instantiates itself: {}",
				prefab.path,
				above
					.iter()
					.chain([&prefab.path])
					.map(|path| format!("`{path}`"))
					.collect::<Vec<_>>()
					.join(" instantiates ")
			);
		}
		let mut above = above;
		above.push(prefab.path.clone());
		for document in prefab.instances.clone() {
			// boxed, since a document's depth is no type
			Box::pin(Self::load_under(
				world,
				package,
				&document,
				above.clone(),
			))
			.await?;
		}
		prefab.xok()
	}

	/// Read the document at `path` and hold it.
	async fn read(
		world: &AsyncWorld,
		package: &PackageStore,
		path: &RelPath,
	) -> Result<Arc<InstancePrefab>> {
		let full = Self::complete(package, path).await?;
		let bytes = package
			.get_media(&full)
			.await
			.map_err(|err| bevyhow!("`{full}`: {err}"))?;
		let (package, path) = (package.clone(), path.clone());
		world
			.with(move |world: &mut World| -> Result<Arc<InstancePrefab>> {
				let body = DocParsers::of(world)
					.body(&bytes)
					.map_err(|err| bevyhow!("`{full}`: {err}"))?;
				let template = body
					.reflect_at(world, &full)
					.map_err(|err| bevyhow!("`{full}`: {err}"))?;
				let prefab = Arc::new(InstancePrefab {
					props: template.props(),
					instances: template.instances(),
					body,
					path: full.clone(),
				});
				let mut cache = world.get_resource_or_init::<Self>();
				for path in [&path, &full] {
					cache.0.insert(
						PrefabKey::new(&package, path),
						CachedPrefab {
							package: package.clone(),
							prefab: prefab.clone(),
						},
					);
				}
				prefab.xok()
			})
			.await
	}

	/// The document `path` names, its extension written: the file itself, else
	/// the one file beside it of its name with an extension.
	async fn complete(
		package: &PackageStore,
		path: &RelPath,
	) -> Result<RelPath> {
		if package.exists(path).await? {
			return path.clone().xok();
		}
		let dir = path.parent().unwrap_or_default();
		let name = path.file_name().unwrap_or_default().to_string();
		let found = package
			.list_dir(&dir)
			.await?
			.files
			.into_iter()
			.map(|file| dir.join(file))
			.filter(|file| file.file_stem() == Some(name.as_str()))
			.collect::<Vec<_>>();
		match found.as_slice() {
			[found] => found.clone().xok(),
			[] => {
				bevybail!("no document at `{path}` in `{}`", package.describe())
			}
			found => bevybail!(
				"`{path}` names several documents, write its extension: {}",
				found
					.iter()
					.map(|path| format!("`{path}`"))
					.collect::<Vec<_>>()
					.join(", ")
			),
		}
	}

	/// Forget every document `event` changed, and every one read by a path
	/// that now may name another file.
	fn forget(&mut self, event: &BlobEvent) {
		self.0.retain(|_, cached| {
			!cached.package.matches_object(event, &cached.prefab.path)
		});
	}
}

/// Forget a document its store says changed.
pub(crate) fn forget_changed_prefabs(
	ev: On<BlobEvent>,
	mut cache: ResMut<PrefabCache>,
) {
	cache.forget(&ev);
}
