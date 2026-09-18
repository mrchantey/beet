//! The template mounting every `secrets` verb.

use super::*;
use beet_core::prelude::*;

/// `<SecretsRoutes/>`: every `secrets` verb as children, authored under
/// `<Route path="secrets">` beside an entry's other commands. The tty verbs
/// (`backup`, `restore-identity`) are native; the rest run wherever a store
/// does.
///
/// ```bsx
/// <Route path="secrets" bx:cfg="feature:secrets"><SecretsRoutes/></Route>
/// ```
#[template]
pub fn SecretsRoutes() -> impl Bundle {
	Children::spawn((
		Spawn(SecretsKeygen),
		Spawn(SecretsCheck),
		Spawn(SecretsEncrypt),
		Spawn(SecretsDecrypt),
		Spawn(SecretsRekey),
		SpawnWith(|spawner: &mut ChildSpawner| {
			#[cfg(not(target_arch = "wasm32"))]
			{
				spawner.spawn(SecretsBackup);
				spawner.spawn(SecretsRestoreIdentity);
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
		let mut expected =
			vec!["check", "decrypt", "encrypt", "keygen", "rekey"];
		#[cfg(not(target_arch = "wasm32"))]
		expected.extend(["backup", "restore-identity"]);
		expected.sort();
		paths.xpect_eq(expected);
	}
}
