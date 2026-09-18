//! The template mounting every `vault` verb.

use super::*;
use beet_core::prelude::*;

/// `<VaultRoutes/>`: every `vault` verb as children, authored under
/// `<Route path="vault">` beside an entry's other commands. The tty verbs
/// (`backup`, `restore-identity`) are native; the rest run wherever a store
/// does.
///
/// ```bsx
/// <Route path="vault" bx:cfg="feature:vault"><VaultRoutes/></Route>
/// ```
#[template]
pub fn VaultRoutes() -> impl Bundle {
	Children::spawn((
		Spawn(VaultKeygen),
		Spawn(VaultEncrypt),
		Spawn(VaultDecrypt),
		Spawn(VaultRekey),
		SpawnWith(|spawner: &mut ChildSpawner| {
			#[cfg(not(target_arch = "wasm32"))]
			{
				spawner.spawn(VaultBackup);
				spawner.spawn(VaultRestoreIdentity);
			}
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
		let root = world.spawn_template(VaultRoutes).unwrap().id();
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
		let mut expected = vec!["decrypt", "encrypt", "keygen", "rekey"];
		#[cfg(not(target_arch = "wasm32"))]
		expected.extend(["backup", "restore-identity"]);
		expected.sort();
		paths.xpect_eq(expected);
	}
}
