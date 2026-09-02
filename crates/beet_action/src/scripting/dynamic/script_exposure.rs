//! What a world-bridged script is allowed to reach.
use crate::prelude::*;
use beet_core::prelude::*;

/// The components a world-bridged script may read and write.
///
/// Open by default and restrictable per script: an absent or default exposure
/// lets a script address anything, and a scene running a script it trusts less
/// names exactly what that script may touch. Enforcement is at the bridge, per
/// call, never in the script: a read or an `entities` query checks
/// [`read`](Self::read), a mutation checks [`write`](Self::write), and a spawn
/// checks every component it carries. A refusal rejects the awaiting promise
/// naming the identifier, so the script can catch it.
///
/// Both halves are a [`GlobFilter`], so a pattern reaches a family of
/// components and an exclude carves one out of an otherwise open grant:
///
/// ```ignore
/// <DynamicScript
///   {ScriptExposure{read:["guestbook.*","Text"], write:["guestbook.*"]}}
///   script=".."/>
/// ```
///
/// A component passes a filter if *any* of its names (full type path, short
/// path, or a runtime component's declared name) matches an include, and *none*
/// of them matches an exclude. So a human writes `"Name"` and the canonical
/// `bevy_ecs::name::Name` still passes, while an exclude cannot be sidestepped
/// by spelling the same component the other way.
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct ScriptExposure {
	/// What the script may read. Empty includes read everything.
	pub read: GlobFilter,
	/// What the script may write. Empty includes write everything.
	pub write: GlobFilter,
}

impl ScriptExposure {
	/// The components no script may ever write, whatever its exposure says.
	///
	/// A script's grant is carried on the entity beside it, so a script that
	/// could write these could widen its own reach or replace its own body: the
	/// sandbox would hold only until the first script asked it not to. This is a
	/// rule rather than a flag, so there is nothing to forget to set.
	///
	/// Matched by short name rather than by type, because one of the carriers
	/// ([`DynamicScriptExchange`], the route form) lives in `beet_router`, which
	/// depends on this crate. A runtime component that picks one of these names
	/// is refused too, which is the conservative direction.
	///
	/// [`DynamicScriptExchange`]: https://docs.rs/beet_router
	pub const PROTECTED: [&str; 3] =
		["ScriptExposure", "DynamicScript", "DynamicScriptExchange"];

	/// An exposure over the named components, readable and writable.
	///
	/// The common case: a script that changes what it reads.
	pub fn new<S: AsRef<str>>(components: impl IntoIterator<Item = S>) -> Self {
		let components = components
			.into_iter()
			.map(|component| component.as_ref().to_string())
			.collect::<Vec<_>>();
		Self {
			read: GlobFilter::default().extend_include(&components),
			write: GlobFilter::default().extend_include(&components),
		}
	}

	/// Narrow the writable set, leaving the readable set alone.
	pub fn with_write<S: AsRef<str>>(
		mut self,
		components: impl IntoIterator<Item = S>,
	) -> Self {
		self.write = GlobFilter::default().extend_include(
			components
				.into_iter()
				.map(|component| component.as_ref().to_string())
				.collect::<Vec<_>>(),
		);
		self
	}

	/// Refuse every write, whatever this exposure reads.
	///
	/// The observer's grip: a script that reads the scene to decide something,
	/// and has no business changing it. Spelled as an exclude of everything, so
	/// it is the same one mechanism rather than a second flag.
	pub fn read_only(mut self) -> Self {
		self.write = GlobFilter::default().with_exclude("*");
		self
	}

	/// Check a resolved identifier against the readable set.
	///
	/// # Errors
	/// Errors naming the identifier when this exposure excludes it.
	pub fn assert_readable(&self, ident: &ComponentIdent) -> Result {
		match Self::passes(&self.read, ident) {
			true => Ok(()),
			false => bevybail!(
				"script may not read `{}`: its read exposure is {}",
				ident.path,
				Self::describe(&self.read)
			),
		}
	}

	/// Check a resolved identifier against the writable set, and against the
	/// unconditional [`PROTECTED`](Self::PROTECTED) rule.
	///
	/// # Errors
	/// Errors naming the identifier when this exposure excludes it, or when it
	/// is one a script may never write.
	pub fn assert_writable(&self, ident: &ComponentIdent) -> Result {
		if Self::PROTECTED.contains(&ident.short.as_str()) {
			bevybail!(
				"no script may write `{}`: it is what grants the script its reach, \
so writing it would let a script widen its own",
				ident.path
			);
		}
		match Self::passes(&self.write, ident) {
			true => Ok(()),
			false => bevybail!(
				"script may not write `{}`: its write exposure is {}",
				ident.path,
				Self::describe(&self.write)
			),
		}
	}

	/// A filter on one line, for the error a script reads back.
	///
	/// A half with no patterns is dropped rather than printed empty, and a
	/// filter with neither is the open grant it is.
	fn describe(filter: &GlobFilter) -> String {
		let described = filter
			.to_string()
			.lines()
			.filter(|half| !half.trim_end().ends_with(':'))
			.collect::<Vec<_>>()
			.join(", ");
		match described.is_empty() {
			true => "everything".to_string(),
			false => described,
		}
	}

	/// Whether `ident` passes `filter` under any of its names.
	///
	/// Include is any-of and exclude is none-of, deliberately asymmetric: a
	/// grant should be easy to spell (`"Name"` reaches the canonical path),
	/// while a denial must not be escapable by spelling the same component
	/// differently.
	fn passes(filter: &GlobFilter, ident: &ComponentIdent) -> bool {
		let names = [ident.path.as_str(), ident.short.as_str()];
		names.iter().any(|name| filter.passes_include(name))
			&& names.iter().all(|name| filter.passes_exclude(name))
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::scripting::dynamic::test_support::*;
	use beet_core::prelude::*;

	/// Resolve `ident` and check it against `exposure`, the shape every write
	/// takes.
	fn writable(exposure: &ScriptExposure, ident: &str) -> Result {
		let mut world = test_world();
		exposure.assert_writable(&ComponentIdent::resolve(&mut world, ident)?)
	}

	/// The read half of [`writable`].
	fn readable(exposure: &ScriptExposure, ident: &str) -> Result {
		let mut world = test_world();
		exposure.assert_readable(&ComponentIdent::resolve(&mut world, ident)?)
	}

	#[beet_core::test]
	fn a_default_exposure_allows_everything() {
		writable(&ScriptExposure::default(), "Name").unwrap();
		readable(&ScriptExposure::default(), "Name").unwrap();
	}

	#[beet_core::test]
	fn a_declared_exposure_allows_what_it_names() {
		writable(&ScriptExposure::new(["Name"]), "Name").unwrap();
	}

	/// The exposure is written the way a human writes it, and a read hands the
	/// script the canonical path, so a script writing back the key it read must
	/// be allowed. This is the trap plain string comparison walks into.
	#[beet_core::test]
	fn a_short_path_exposure_permits_the_canonical_path() {
		writable(&ScriptExposure::new(["Name"]), "bevy_ecs::name::Name")
			.unwrap();
	}

	#[beet_core::test]
	fn a_full_path_exposure_permits_the_short_path() {
		writable(&ScriptExposure::new(["bevy_ecs::name::Name"]), "Name")
			.unwrap();
	}

	#[beet_core::test]
	fn a_glob_reaches_a_family() {
		let mut world = test_world();
		DynamicComponents::register(
			&mut world,
			"guestbook.Visits",
			ValueSchema::Any,
		)
		.unwrap();
		ScriptExposure::new(["guestbook.*"])
			.assert_writable(
				&ComponentIdent::resolve(&mut world, "guestbook.Visits")
					.unwrap(),
			)
			.unwrap();
	}

	/// An exclude carves one component out of an otherwise open grant, and
	/// applies under every name that component answers to.
	#[beet_core::test]
	fn an_exclude_holds_under_either_name() {
		let exposure = ScriptExposure {
			write: GlobFilter::default().with_exclude("*name::Name"),
			..default()
		};
		writable(&exposure, "Name")
			.unwrap_err()
			.to_string()
			.xpect_contains("may not write `bevy_ecs::name::Name`");
		writable(&exposure, "bevy_ecs::name::Name").unwrap_err();
	}

	#[beet_core::test]
	fn narrowing_writes_keeps_reads() {
		let exposure = ScriptExposure::new(["Name", "game.Health"])
			.with_write(["game.Health"]);
		readable(&exposure, "Name").unwrap();
		writable(&exposure, "Name")
			.unwrap_err()
			.to_string()
			.xpect_contains("may not write");
	}

	#[beet_core::test]
	fn read_only_refuses_every_write() {
		let exposure = ScriptExposure::new(["Name"]).read_only();
		readable(&exposure, "Name").unwrap();
		writable(&exposure, "Name").unwrap_err();
	}

	/// The self-protection rule holds against the most open exposure there is,
	/// because it is a rule and not a filter.
	#[beet_core::test]
	fn no_exposure_can_grant_a_write_to_itself() {
		let mut world = test_world();
		world
			.resource::<AppTypeRegistry>()
			.write()
			.register::<ScriptExposure>();
		ScriptExposure::default()
			.assert_writable(
				&ComponentIdent::resolve(&mut world, "ScriptExposure").unwrap(),
			)
			.unwrap_err()
			.to_string()
			.xpect_contains("no script may write");
	}

	/// The same rule guards the script's own body, so a script cannot replace
	/// what runs next time.
	#[beet_core::test]
	fn no_exposure_can_grant_a_write_to_the_script_itself() {
		let mut world = test_world();
		world
			.resource::<AppTypeRegistry>()
			.write()
			.register::<DynamicScript>();
		ScriptExposure::default()
			.assert_writable(
				&ComponentIdent::resolve(&mut world, "DynamicScript").unwrap(),
			)
			.unwrap_err()
			.to_string()
			.xpect_contains("no script may write");
	}
}
