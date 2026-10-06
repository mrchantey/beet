//! Resolving the repo an entity reads and writes.
use crate::prelude::*;
use beet_core::prelude::*;

/// Names an account declared elsewhere in the document, so an entity outside
/// the account's subtree reaches its [`Pds`]: `{AccountRef($beet_org)}`
/// beside an `<AtprotoAccount bx:ref="beet_org" ../>`.
#[derive(
	Debug, Clone, Copy, PartialEq, Eq, Component, Reflect, MapEntities,
)]
#[reflect(Component, MapEntities)]
pub struct AccountRef(#[entities] pub Entity);

/// The [`Pds`] an entity reads and writes: walking from the entity up, the
/// first that carries a `Pds` (an `<AtprotoAccount/>` or any other provider)
/// or an [`AccountRef`] naming one wins.
#[derive(SystemParam)]
pub struct PdsQuery<'w, 's> {
	parents: Query<'w, 's, &'static ChildOf>,
	repos: Query<'w, 's, &'static Pds>,
	refs: Query<'w, 's, &'static AccountRef>,
}

impl PdsQuery<'_, '_> {
	/// The repo `entity` resolves to, shaped to pass directly to
	/// [`AsyncEntity::with_world`].
	pub fn resolve(world: &mut World, entity: Entity) -> Result<Pds> {
		world.with_state::<PdsQuery, _>(|query| query.get(entity))
	}

	/// The repo `entity` resolves to; an error naming the declaration it
	/// needs when there is none, or when a referenced account has not landed
	/// its repo.
	pub fn get(&self, entity: Entity) -> Result<Pds> {
		let mut current = Some(entity);
		while let Some(entity) = current {
			if let Ok(pds) = self.repos.get(entity) {
				return pds.clone().xok();
			}
			if let Ok(AccountRef(account)) = self.refs.get(entity) {
				return self.repos.get(*account).cloned().map_err(|_| {
					bevyhow!(
						"entity {entity} names account {account}, which has no \
						 repo: it must be an `<AtprotoAccount/>`, and this build \
						 must link the `atproto` feature to reach it"
					)
				});
			}
			current = self.parents.get(entity).ok().map(ChildOf::parent);
		}
		bevybail!(
			"entity {entity} has no repo: declare it under an \
			 `<AtprotoAccount/>`, or name one with `{{AccountRef($account)}}`"
		)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn resolves_by_ancestry_then_reference() {
		let mut world = World::new();
		let account = world.spawn(Pds::temp()).id();
		let child = world.spawn(ChildOf(account)).id();
		let elsewhere = world.spawn(AccountRef(account)).id();
		let nested = world.spawn(ChildOf(elsewhere)).id();
		let orphan = world.spawn_empty().id();
		for entity in [account, child, elsewhere, nested] {
			PdsQuery::resolve(&mut world, entity)
				.unwrap()
				.id()
				.xpect_eq(EmulatorPds::ID);
		}
		PdsQuery::resolve(&mut world, orphan)
			.unwrap_err()
			.to_string()
			.xpect_contains("AccountRef");
	}
}
