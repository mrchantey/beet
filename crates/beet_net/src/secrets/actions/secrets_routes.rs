//! The template mounting every `secrets` verb.

use super::*;
use beet_core::prelude::*;

/// `<SecretsRoutes/>`: every `secrets` verb as children, authored under
/// `<Route path="secrets">` beside an entry's other commands, with a
/// `<Secrets/>` declaration beside it naming the document the verbs default
/// to. `exec` is native; the rest run wherever a store does. The identity
/// and age file verbs are `<VaultRoutes/>`.
///
/// ```bsx
/// <Secrets/>
/// <Route path="secrets" bx:cfg="feature:vault"><SecretsRoutes/></Route>
/// ```
///
/// The declaration itself carries no `bx:cfg`: the launch reads it out of the
/// entry prescan before the build, and a lean build keeps it as an inert tag.
#[template]
pub fn SecretsRoutes() -> impl Bundle {
	Children::spawn((
		Spawn(SecretsCheck),
		Spawn(SecretsLs),
		Spawn(SecretsGet),
		Spawn(SecretsSet),
		Spawn(SecretsRm),
		Spawn(SecretsRekey),
		SpawnWith(|spawner: &mut ChildSpawner| {
			#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
			spawner.spawn(SecretsExec);
			// nothing native to mount in this build
			let _ = spawner;
		}),
	))
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::prelude::*;

	/// Every verb mounts under its path.
	#[beet_core::test]
	fn mounts_every_verb() {
		let mut world = World::new();
		let root = world.spawn_template(SecretsRoutes).unwrap().id();
		world.flush();
		let mut paths = world
			.query::<(&PathPartial, &ChildOf)>()
			.iter(&world)
			.filter(|(_, child_of)| child_of.parent() == root)
			.map(|(path, _)| {
				path.segments
					.iter()
					.map(ToString::to_string)
					.collect::<Vec<_>>()
					.join("/")
			})
			.collect::<Vec<_>>();
		paths.sort();
		// a `:name` segment displays bare
		let mut expected =
			vec!["check", "get/name", "ls", "rekey", "rm/name", "set/name"];
		#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
		expected.push("exec");
		expected.sort();
		paths.xpect_eq(expected);
	}
}
