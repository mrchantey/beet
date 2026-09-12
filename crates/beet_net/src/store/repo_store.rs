//! The repo store: the one canonical store an app runs from.

use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;

/// Marks the entity carrying the app's **repo store**, the single canonical
/// [`BlobStore`] an entry loads through: the entry document, its templates,
/// routes and assets all live in it, and a relative path anywhere in the app
/// means a path in it.
///
/// A repo store is to content what a router is to urls, with one difference
/// that matters: an app may declare many routers, but only ever one repo store.
/// Inserting a second anywhere in the world is an error, so "the repo store" is
/// always unambiguous. Every other [`BlobStore`] is a plain store, declared for
/// a purpose and reached by name through a `StoreRef` or scoped out of an
/// ancestor by a [`DirPath`].
///
/// Consumers usually resolve it by ancestry (the `AncestorQuery<&BlobStore>`
/// idiom), which finds the nearest store and so honours any intervening scope;
/// [`RepoStore::get`] is the direct lookup for code that wants *the* repo store
/// wherever it sits.
///
/// It marks the store rather than holding it, since [`BlobStore`] erases a
/// backend that no scene can serialize: the marker travels with a document, the
/// store is composed by whichever driver loaded it. Only a driver build (the
/// process's own entry) claims it, so a command loading a foreign entry into the
/// same world roots that sub-app's store unmarked, reachable by its own
/// ancestry.
///
/// Both invariants are enforced by its own insert hook, at the next flush: one
/// per world ([`hook_ext::exclusive_with`]) and a [`BlobStore`] beside it.
#[derive(Debug, Default, Clone, Copy, PartialEq, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_insert = hook_ext::chain(
	hook_ext::exclusive_with::<RepoStore>(
		"tear the previous entry scene down before building the next"
	),
	RepoStore::assert_store
))]
pub struct RepoStore;

impl RepoStore {
	/// The app's repo store, erroring when no entity carries one.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// let mut world = StorePlugin.into_world();
	/// world.spawn((BlobStore::temp(), RepoStore));
	/// RepoStore::get(&mut world).unwrap();
	/// ```
	pub fn get(world: &mut World) -> Result<BlobStore> {
		let mut query = world.query_filtered::<&BlobStore, With<RepoStore>>();
		match query.single(world) {
			Ok(store) => store.clone().xok(),
			// the singleton is enforced on insert, so absence is the only failure
			Err(_) => bevybail!(
				"no repo store in this world: an entry load composes one on its \
				root, ie `(store, RepoStore)`"
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
				entity_ref.contains::<RepoStore>(),
				entity_ref.contains::<BlobStore>(),
			) {
				(true, false) => bevybail!(
					"entity {entity} is marked `RepoStore` but carries no \
					 `BlobStore`: insert the store in the same bundle, ie \
					 `(store, RepoStore)`"
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

	/// The repo store resolves wherever it sits, without walking ancestry.
	#[beet_core::test]
	fn resolves() {
		let mut world = store_world();
		RepoStore::get(&mut world)
			.unwrap_err()
			.to_string()
			.xpect_contains("no repo store");
		world.spawn((children![(BlobStore::temp(), RepoStore)],));
		RepoStore::get(&mut world).unwrap();
	}

	/// A second repo store anywhere in the world is an error.
	#[beet_core::test]
	#[should_panic = "already carries one"]
	fn rejects_second_repo_store() {
		let mut world = store_world();
		world.spawn((BlobStore::temp(), RepoStore));
		world.spawn((BlobStore::temp(), RepoStore));
		world.flush();
	}

	/// The marker without the store it claims is an error.
	#[beet_core::test]
	#[should_panic = "carries no `BlobStore`"]
	fn rejects_storeless_marker() {
		let mut world = store_world();
		world.spawn(RepoStore);
		world.flush();
	}
}
