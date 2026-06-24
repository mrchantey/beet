//! Asset deferral for the template substrate, behind the `bevy_asset` feature.
//!
//! An asset handle is a templated value: the template is the asset path, and
//! building it resolves the path to a [`Handle`] through the [`AssetServer`].
//! This is the canonical example of why a [`Template`] carries world context.
//!
//! [`LoadTemplate`] itself is core and fires immediately when nothing is pending
//! (see the lifecycle). What this feature adds is the deferral: an asset
//! produced anywhere in a subtree registers a pending dependency on the template
//! root, and [`drain_loaded_assets`] resolves it when the asset finishes loading,
//! firing [`LoadTemplate`] only once every tracked asset has settled.
//!
//! Removing the feature leaves a fully functional asset-free substrate: the
//! no_std core representation, walker, and value-slot serde never reference an
//! asset type.

use crate::prelude::*;
use bevy::asset::Asset;
use bevy::asset::AssetServer;
use bevy::asset::Handle;
use bevy::asset::RecursiveDependencyLoadState;
use bevy::asset::UntypedHandle;
use bevy::ecs::system::SystemParam;
use bevy::ecs::template::Template;
use bevy::ecs::template::TemplateContext;

/// A [`Template`] that resolves an asset path to a strong [`Handle`] and defers
/// [`LoadTemplate`] until the asset (and its dependencies) finish loading.
///
/// Build loads the handle, then registers a pending dependency on the template
/// root, tracked by [`PendingAssets`] and drained by [`drain_loaded_assets`].
pub struct AssetLoadTemplate<A: Asset> {
	/// The asset path to load.
	pub path: SmolStr,
	_marker: core::marker::PhantomData<fn() -> A>,
}

impl<A: Asset> AssetLoadTemplate<A> {
	/// A template that loads the asset at `path`.
	pub fn new(path: impl Into<SmolStr>) -> Self {
		Self {
			path: path.into(),
			_marker: core::marker::PhantomData,
		}
	}
}

impl<A: Asset> Template for AssetLoadTemplate<A> {
	type Output = Handle<A>;

	/// Loads the handle and registers it as a pending dependency on the root.
	fn build_template(&self, cx: &mut TemplateContext) -> Result<Handle<A>> {
		let handle = cx
			.entity
			.resource::<AssetServer>()
			.load::<A>(self.path.to_string());

		// the root carries the pending set; fall back to this entity if no
		// surrounding template build set a root.
		let entity_id = cx.entity.id();
		let root = cx
			.entity
			.world_scope(|world| TemplateBuildRoot::resolve(world, entity_id));

		// SAFETY: only used to register the pending dependency on the root.
		let world = unsafe { cx.entity.world_mut() };
		let mut root_entity = world.entity_mut(root);
		let pending_id = root_entity
			.entry::<TemplatePending>()
			.or_default()
			.get_mut()
			.register();
		// keep a strong handle alongside the id so the load is not cancelled by
		// the handle returned here being dropped before the asset finishes.
		root_entity
			.entry::<PendingAssets>()
			.or_default()
			.get_mut()
			.push(handle.clone().untyped(), pending_id);

		Ok(handle)
	}

	fn clone_template(&self) -> Self {
		Self {
			path: self.path.clone(),
			_marker: core::marker::PhantomData,
		}
	}
}

/// A [`SystemParam`] for loading assets from inside a `#[template(system)]`, so
/// the load defers [`LoadTemplate`] until the asset finishes loading. The
/// system-side counterpart of [`AssetLoadTemplate`].
///
/// A raw `asset_server.load(..)` inside a template mints a handle but lets
/// `LoadTemplate` fire immediately; `BuildAssets::load` parks a pending
/// dependency on the build root instead, so behaviours never run before their
/// assets exist.
#[derive(SystemParam)]
pub struct BuildAssets<'w, 's> {
	server: Res<'w, AssetServer>,
	build_root: Option<Res<'w, TemplateBuildRoot>>,
	commands: Commands<'w, 's>,
}

impl BuildAssets<'_, '_> {
	/// Load the asset at `path`, registering it as a pending dependency on the
	/// current template build root so `LoadTemplate` defers until it loads.
	///
	/// Outside a template build (no [`TemplateBuildRoot`]) it loads without
	/// deferral, like a plain `asset_server.load`.
	pub fn load<A: Asset>(&mut self, path: impl Into<String>) -> Handle<A> {
		let handle = self.server.load::<A>(path.into());
		let Some(root) = self.build_root.as_deref().map(|root| **root) else {
			return handle;
		};
		// register synchronously (the queue drains in `with_state::apply`, before
		// the build's `drain_pending_dependencies`), keeping a strong handle so the
		// load is not cancelled before it settles.
		let untyped = handle.clone().untyped();
		self.commands.queue(move |world: &mut World| {
			let mut root = world.entity_mut(root);
			let id = root
				.entry::<TemplatePending>()
				.or_default()
				.get_mut()
				.register();
			root.entry::<PendingAssets>()
				.or_default()
				.get_mut()
				.push(untyped, id);
		});
		handle
	}
}

/// Tracks the outstanding asset dependencies registered on a template root: each
/// pairs a strong asset handle (kept alive so the load is not cancelled) with the
/// [`PendingId`] it parked on [`TemplatePending`].
#[derive(Debug, Default, Component)]
pub struct PendingAssets(Vec<(UntypedHandle, PendingId)>);

impl PendingAssets {
	/// Track a strong handle against the [`PendingId`] it parked on the root.
	fn push(&mut self, handle: UntypedHandle, id: PendingId) {
		self.0.push((handle, id));
	}
}

/// Registers [`drain_loaded_assets`] so a deferred asset load (via
/// [`AssetLoadTemplate`] or [`BuildAssets`]) resolves once the asset finishes
/// loading. Add to any running app whose templates load assets.
#[derive(Default)]
pub struct AssetTemplatePlugin;

impl Plugin for AssetTemplatePlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, drain_loaded_assets);
	}
}

/// Resolves loaded (or failed) tracked assets and drains the root's pending set.
///
/// Each frame, for every root with outstanding [`PendingAssets`], an asset whose
/// recursive dependency load state is settled (loaded or failed) is resolved on
/// [`TemplatePending`] and dropped from tracking; when the set drains,
/// [`drain_pending_dependencies`] fires [`LoadTemplate`].
pub fn drain_loaded_assets(world: &mut World) {
	let roots = world
		.query_filtered::<Entity, With<PendingAssets>>()
		.iter(world)
		.collect::<Vec<_>>();

	for root in roots {
		// partition tracked assets into settled (to resolve) and still pending.
		let asset_server = world.resource::<AssetServer>().clone();
		let pending = core::mem::take(
			&mut world.entity_mut(root).get_mut::<PendingAssets>().unwrap().0,
		);
		let mut settled = Vec::new();
		let mut still_pending = Vec::new();
		for (handle, pending_id) in pending {
			match asset_server.get_recursive_dependency_load_state(handle.id())
			{
				Some(RecursiveDependencyLoadState::Loaded)
				| Some(RecursiveDependencyLoadState::Failed(_)) => {
					settled.push(pending_id);
				}
				_ => still_pending.push((handle, pending_id)),
			}
		}

		let mut root_entity = world.entity_mut(root);
		root_entity.get_mut::<PendingAssets>().unwrap().0 = still_pending;
		if settled.is_empty() {
			continue;
		}
		// resolve each settled dependency, then drain (fires LoadTemplate iff empty).
		if let Some(mut template_pending) =
			root_entity.get_mut::<TemplatePending>()
		{
			for pending_id in settled {
				template_pending.resolve(pending_id);
			}
		}
		drain_pending_dependencies(&mut root_entity);
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use bevy::asset::AssetApp;
	use bevy::asset::AssetPlugin;
	use bevy::asset::io::AssetSourceBuilder;
	use bevy::asset::io::AssetSourceId;
	use bevy::asset::io::memory::Dir;
	use bevy::asset::io::memory::MemoryAssetReader;

	/// A trivial loadable asset backed by a UTF-8 source file.
	#[derive(Asset, TypePath, Debug)]
	struct TextAsset(#[allow(dead_code)] String);

	#[derive(Default, TypePath)]
	struct TextLoader;
	impl bevy::asset::AssetLoader for TextLoader {
		type Asset = TextAsset;
		type Settings = ();
		type Error = std::io::Error;
		async fn load(
			&self,
			reader: &mut dyn bevy::asset::io::Reader,
			_settings: &(),
			_load_context: &mut bevy::asset::LoadContext<'_>,
		) -> std::result::Result<TextAsset, std::io::Error> {
			let mut bytes = Vec::new();
			bevy::asset::io::Reader::read_to_end(reader, &mut bytes).await?;
			Ok(TextAsset(String::from_utf8_lossy(&bytes).into_owned()))
		}
		fn extensions(&self) -> &[&str] { &["txt"] }
	}

	/// An app with an in-memory asset source holding one `hello.txt`.
	fn asset_app() -> App {
		let dir = Dir::default();
		dir.insert_asset_text(std::path::Path::new("hello.txt"), "hi");

		let mut app = App::new();
		app.add_plugins((MinimalPlugins, TemplatePlugin));
		app.register_asset_source(
			AssetSourceId::Default,
			AssetSourceBuilder::new(move || {
				Box::new(MemoryAssetReader { root: dir.clone() })
			}),
		);
		app.add_plugins(AssetPlugin::default());
		app.init_asset::<TextAsset>();
		app.register_asset_loader(TextLoader);
		app.add_systems(Update, drain_loaded_assets);
		app
	}

	#[crate::test(timeout_ms = 5000)]
	async fn defers_load_until_asset_loaded() {
		let mut app = asset_app();
		let world = app.world_mut();

		let load_state = Store::new(None);
		let ls = load_state.clone();
		world.add_observer(move |ev: On<LoadTemplate>| {
			ls.set(Some(ev.is_error))
		});

		// a template that loads the asset, deferring LoadTemplate.
		let root = world
			.spawn_template(bevy::ecs::template::template(
				|cx: &mut TemplateContext| {
					AssetLoadTemplate::<TextAsset>::new("hello.txt")
						.build_template(cx)?;
					OK
				},
			))
			.unwrap()
			.id();

		// LoadTemplate has not fired: the asset is still pending.
		load_state.get().xpect_none();
		app.world()
			.entity(root)
			.contains::<PendingAssets>()
			.xpect_true();

		// pump frames until the asset loads and the pending set drains; `update_until`
		// ticks the async runtime between frames so the in-memory IO task progresses.
		app_ext::update_until(&mut app, |_world| load_state.get().is_some())
			.await
			.xpect_true();
		// LoadTemplate fired, no error, once the asset finished loading.
		load_state.get().xpect_eq(Some(false));
	}

	/// `BuildAssets::load` (the system-side helper) parks the load as a pending
	/// dependency on the build root, so `LoadTemplate` defers. The full
	/// load-then-fire cycle is covered by [`defers_load_until_asset_loaded`];
	/// both drain through the same [`drain_loaded_assets`].
	#[beet_core::test]
	fn build_assets_defers_load() {
		let mut app = asset_app();
		let world = app.world_mut();

		let fired = Store::new(false);
		let f = fired.clone();
		world.add_observer(move |_: On<LoadTemplate>| f.set(true));

		// a `#[template(system)]`-style build that loads through `BuildAssets`.
		let root = world
			.spawn_template(system_template::<BuildAssets, _, _>(
				|_entity, mut assets: BuildAssets| {
					assets.load::<TextAsset>("hello.txt");
					Snippet::from_bundle(())
				},
			))
			.unwrap()
			.id();

		// LoadTemplate deferred: the asset is parked pending on the build root.
		fired.get().xpect_false();
		app.world()
			.entity(root)
			.contains::<PendingAssets>()
			.xpect_true();
	}
}
