//! The package store: the package a runtime runs from.

use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;

/// Marks the entity carrying the runtime's **package store**, the single
/// canonical [`BlobStore`] an entry loads through: the package a runtime runs,
/// its entry document, templates, routes and assets, so a relative path
/// anywhere in the runtime means a path in it. A package is its sources and
/// its compiled records alike, so this is a source directory, a bucket synced
/// from one, or a package's space.
///
/// A package store is to content what a router is to urls, with one
/// difference that matters: a runtime may declare many routers, but only ever
/// one package store. Inserting a second anywhere in the world is an error, so
/// "the package store" is always unambiguous. Every other [`BlobStore`] is a
/// plain store, declared for a purpose and reached by name through a
/// `StoreRef` or scoped out of an ancestor by a [`DirPath`]; the files a
/// [`DocPath`] names are [`PackageFiles`], scoped to the entry's directory.
///
/// Consumers usually resolve it by ancestry (the `AncestorQuery<&BlobStore>`
/// idiom), which finds the nearest store and so honours any intervening scope;
/// [`PackageStore::get`] is the direct lookup for code that wants *the*
/// package store wherever it sits.
///
/// It marks the store rather than holding it, since [`BlobStore`] erases a
/// backend that no scene can serialize: the marker travels with a document,
/// the store is composed by whichever driver loaded it. Only a driver build
/// (the process's own entry) claims it, so a command loading a foreign entry
/// into the same world roots that entry's store unmarked, reachable by its own
/// ancestry.
///
/// Both invariants are enforced by its own insert hook, at the next flush: one
/// per world ([`hook_ext::exclusive_with`]) and a [`BlobStore`] beside it.
#[derive(Debug, Default, Clone, Copy, PartialEq, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_insert = hook_ext::chain(
	hook_ext::exclusive_with::<PackageStore>(
		"tear the previous entry scene down before building the next"
	),
	PackageStore::assert_store
))]
pub struct PackageStore;

impl PackageStore {
	/// The runtime's package store, erroring when no entity carries one.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// let mut world = StorePlugin.into_world();
	/// world.spawn((BlobStore::temp(), PackageStore));
	/// PackageStore::get(&mut world).unwrap();
	/// ```
	pub fn get(world: &mut World) -> Result<BlobStore> {
		let mut query =
			world.query_filtered::<&BlobStore, With<PackageStore>>();
		match query.single(world) {
			Ok(store) => store.clone().xok(),
			// the singleton is enforced on insert, so absence is the only failure
			Err(_) => bevybail!(
				"no package store in this world: an entry load composes one on its \
				root, ie `(store, PackageStore)`"
			),
		}
	}

	/// Insert hook: the marker claims a [`BlobStore`] on the same entity,
	/// checked at the next flush so a bundle's other components have landed.
	fn assert_store(mut world: DeferredWorld, cx: HookContext) {
		let entity = cx.entity;
		world.commands().queue(move |world: &mut World| -> Result {
			// removed or despawned before the flush: nothing left to claim
			let Ok(entity_ref) = world.get_entity(entity) else {
				return Ok(());
			};
			match (
				entity_ref.contains::<PackageStore>(),
				entity_ref.contains::<BlobStore>(),
			) {
				(true, false) => bevybail!(
					"entity {entity} is marked `PackageStore` but carries no \
					 `BlobStore`: insert the store in the same bundle, ie \
					 `(store, PackageStore)`"
				),
				_ => Ok(()),
			}
		});
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	fn store_world() -> World { StorePlugin.into_world() }

	/// The package store resolves wherever it sits, without walking ancestry.
	#[beet_core::test]
	fn resolves() {
		let mut world = store_world();
		PackageStore::get(&mut world)
			.unwrap_err()
			.to_string()
			.xpect_contains("no package store");
		world.spawn((children![(BlobStore::temp(), PackageStore)],));
		PackageStore::get(&mut world).unwrap();
	}

	/// A second package store anywhere in the world is an error.
	#[beet_core::test]
	#[should_panic = "already carries one"]
	fn rejects_second_package_store() {
		let mut world = store_world();
		world.spawn((BlobStore::temp(), PackageStore));
		world.spawn((BlobStore::temp(), PackageStore));
		world.flush();
	}

	/// The marker without the store it claims is an error.
	#[beet_core::test]
	#[should_panic = "carries no `BlobStore`"]
	fn rejects_storeless_marker() {
		let mut world = store_world();
		world.spawn(PackageStore);
		world.flush();
	}
}
