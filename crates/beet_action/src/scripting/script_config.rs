//! What the host grants a running [`Script`]: world access, console, reach and
//! resources.
use crate::prelude::*;
use beet_core::prelude::*;
use serde::Serialize;

/// Everything a [`Script`] is granted, beside the program itself.
///
/// A `Script` is the program and this is the envelope: world access, console
/// access, the components the world bridge will address on its behalf, and the
/// resources it may consume. Non-generic, so a policy can be stamped on a
/// scripted entity, audited or edited without knowing the script's input and
/// output types, and an absent sibling is [`ScriptConfig::default`]: everything
/// on, open filters, default limits.
///
/// A toggle off means the global is simply absent, so a script touching it
/// throws an ordinary catchable `ReferenceError`. "Pure" is not a mode, it is
/// `world: false`.
///
/// The two filters are enforced at the bridge, per call, never in the script: a
/// read or an `entities` query checks [`read`](Self::read), a mutation checks
/// [`write`](Self::write), and a spawn checks every component it carries. A
/// refusal rejects the awaiting promise naming the identifier, so the script can
/// catch it.
///
/// Both halves are a [`GlobFilter`], so a pattern reaches a family of
/// components and an exclude carves one out of an otherwise open grant:
///
/// ```ignore
/// <RunScript
///   {ScriptConfig{read:["guestbook.*","Text"], write:["guestbook.*"]}}
///   script=".."/>
/// ```
///
/// A component passes a filter if *any* of its names (full type path, short
/// path, or a runtime component's declared name) matches an include, and *none*
/// of them matches an exclude. So a human writes `"Name"` and the canonical
/// `bevy_ecs::name::Name` still passes, while an exclude cannot be sidestepped
/// by spelling the same component the other way.
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct ScriptConfig {
	/// Whether to install the `world` bridge global. Default true; false makes
	/// the script provably pure, with no `world` to reach for.
	pub world: bool,
	/// Whether to install the `console` global. Default true.
	pub console: bool,
	/// Components and resources the script may read. Empty includes read everything.
	pub read: GlobFilter,
	/// Components and resources the script may write. Empty includes write everything.
	pub write: GlobFilter,
	/// The resources this script may consume before it is cut off.
	pub limits: ScriptLimits,
}

impl Default for ScriptConfig {
	fn default() -> Self {
		Self {
			world: true,
			console: true,
			read: GlobFilter::default(),
			write: GlobFilter::default(),
			limits: ScriptLimits::default(),
		}
	}
}

impl ScriptConfig {
	/// The components no script may ever write, whatever a config says.
	///
	/// A script's grant is carried on the entity beside it, so a script that
	/// could write these could widen its own reach or replace its own body: the
	/// sandbox would hold only until the first script asked it not to. This is a
	/// rule rather than a flag, so there is nothing to forget to set.
	///
	/// Matched by base name rather than by type: the carriers are generic (so
	/// their short path reads `Script<Value, Value>`) and one of them
	/// ([`ExchangeScript`], the route marker) lives in `beet_router`, which
	/// depends on this crate. A runtime component that picks one of these names
	/// is refused too, which is the conservative direction.
	///
	/// [`ExchangeScript`]: https://docs.rs/beet_router
	pub const PROTECTED: [&str; 4] =
		["ScriptConfig", "Script", "OutcomeScript", "ExchangeScript"];

	/// A config reaching the named components, readable and writable.
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
			..default()
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

	/// Refuse every write, whatever this config reads.
	///
	/// The observer's grip: a script that reads the scene to decide something,
	/// and has no business changing it. Spelled as an exclude of everything, so
	/// it is the same one mechanism rather than a second flag.
	pub fn read_only(mut self) -> Self {
		self.write = GlobFilter::default().with_exclude("*");
		self
	}

	/// Withhold the `world` global, leaving the script a pure transform.
	pub fn without_world(mut self) -> Self {
		self.world = false;
		self
	}

	/// Withhold the `console` global.
	pub fn without_console(mut self) -> Self {
		self.console = false;
		self
	}

	/// Set the resource ceilings the script runs under.
	pub fn with_limits(mut self, limits: ScriptLimits) -> Self {
		self.limits = limits;
		self
	}

	/// Check a resolved identifier against the readable set.
	///
	/// # Errors
	/// Errors naming the identifier when this config excludes it.
	pub fn assert_readable(&self, ident: &ComponentIdent) -> Result {
		match Self::passes(&self.read, ident) {
			true => Ok(()),
			false => bevybail!(
				"script may not read `{}`: its read filter is {}",
				ident.path,
				Self::describe(&self.read)
			),
		}
	}

	/// Check a resolved identifier against the writable set, and against the
	/// unconditional [`PROTECTED`](Self::PROTECTED) rule.
	///
	/// # Errors
	/// Errors naming the identifier when this config excludes it, or when it is
	/// one a script may never write.
	pub fn assert_writable(&self, ident: &ComponentIdent) -> Result {
		// generics are stripped so a carrier is refused under every
		// instantiation: `Script<Value, Value>` is a `Script`.
		let base = ident
			.short
			.split('<')
			.next()
			.unwrap_or(ident.short.as_str());
		if Self::PROTECTED.contains(&base) {
			bevybail!(
				"no script may write `{}`: it is what grants the script its reach, \
so writing it would let a script widen its own",
				ident.path
			);
		}
		match Self::passes(&self.write, ident) {
			true => Ok(()),
			false => bevybail!(
				"script may not write `{}`: its write filter is {}",
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

/// The resource ceilings a [`Script`] runs under.
///
/// Every field is enforced by the embedded engine, which can interrupt and cap a
/// running script directly. A host-realm backend enforces what its host allows
/// and documents the rest as not provided: a sandboxed iframe, for instance,
/// cannot be terminated mid-loop, so its module doc states [`timeout`] as an
/// unenforced guarantee rather than pretending otherwise.
///
/// [`timeout`]: Self::timeout
#[derive(
	Debug, Clone, Copy, PartialEq, Eq, Reflect, Serialize, serde::Deserialize,
)]
#[reflect(Default)]
pub struct ScriptLimits {
	/// Wall-clock budget for the whole evaluation, including any microtasks it
	/// drains. Default 10s.
	pub timeout: Duration,
	/// Maximum bytes the engine may allocate. Default 128MB.
	pub memory: u64,
	/// Maximum interpreter stack in bytes, so runaway recursion becomes a
	/// catchable `RangeError` rather than a host stack overflow. Default 256KB.
	///
	/// Always set explicitly, never left to the engine default: QuickJS defaults
	/// to 1MB, exactly the wasm shadow-stack size, and its `stack_top - 1MB`
	/// arithmetic wraps there, disabling stack checking entirely.
	pub stack: u32,
}

impl Default for ScriptLimits {
	fn default() -> Self {
		Self {
			timeout: Duration::from_secs(10),
			memory: 128 * 1024 * 1024,
			stack: 256 * 1024,
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::scripting::dynamic::test_support::*;
	use beet_core::prelude::*;

	/// Resolve `ident` and check it against `config`, the shape every write
	/// takes.
	fn writable(config: &ScriptConfig, ident: &str) -> Result {
		let mut world = test_world();
		config.assert_writable(&ComponentIdent::resolve(&mut world, ident)?)
	}

	/// The read half of [`writable`].
	fn readable(config: &ScriptConfig, ident: &str) -> Result {
		let mut world = test_world();
		config.assert_readable(&ComponentIdent::resolve(&mut world, ident)?)
	}

	#[beet_core::test]
	fn a_default_config_allows_everything() {
		writable(&ScriptConfig::default(), "Name").unwrap();
		readable(&ScriptConfig::default(), "Name").unwrap();
	}

	#[beet_core::test]
	fn a_declared_config_allows_what_it_names() {
		writable(&ScriptConfig::new(["Name"]), "Name").unwrap();
	}

	/// The config is written the way a human writes it, and a read hands the
	/// script the canonical path, so a script writing back the key it read must
	/// be allowed. This is the trap plain string comparison walks into.
	#[beet_core::test]
	fn a_short_path_config_permits_the_canonical_path() {
		writable(&ScriptConfig::new(["Name"]), "bevy_ecs::name::Name").unwrap();
	}

	#[beet_core::test]
	fn a_full_path_config_permits_the_short_path() {
		writable(&ScriptConfig::new(["bevy_ecs::name::Name"]), "Name").unwrap();
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
		ScriptConfig::new(["guestbook.*"])
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
		let config = ScriptConfig {
			write: GlobFilter::default().with_exclude("*name::Name"),
			..default()
		};
		writable(&config, "Name")
			.unwrap_err()
			.to_string()
			.xpect_contains("may not write `bevy_ecs::name::Name`");
		writable(&config, "bevy_ecs::name::Name").unwrap_err();
	}

	#[beet_core::test]
	fn narrowing_writes_keeps_reads() {
		let config = ScriptConfig::new(["Name", "game.Health"])
			.with_write(["game.Health"]);
		readable(&config, "Name").unwrap();
		writable(&config, "Name")
			.unwrap_err()
			.to_string()
			.xpect_contains("may not write");
	}

	#[beet_core::test]
	fn read_only_refuses_every_write() {
		let config = ScriptConfig::new(["Name"]).read_only();
		readable(&config, "Name").unwrap();
		writable(&config, "Name").unwrap_err();
	}

	/// The self-protection rule holds against the most open config there is,
	/// because it is a rule and not a filter.
	#[beet_core::test]
	fn no_config_can_grant_a_write_to_itself() {
		let mut world = test_world();
		world
			.resource::<AppTypeRegistry>()
			.write()
			.register::<ScriptConfig>();
		ScriptConfig::default()
			.assert_writable(
				&ComponentIdent::resolve(&mut world, "ScriptConfig").unwrap(),
			)
			.unwrap_err()
			.to_string()
			.xpect_contains("no script may write");
	}

	/// The same rule guards the script's own body under every instantiation: the
	/// carrier is generic, so the refusal matches its base name and a script
	/// cannot replace what runs next time.
	#[beet_core::test]
	fn no_config_can_grant_a_write_to_the_script_itself() {
		let mut world = test_world();
		world
			.resource::<AppTypeRegistry>()
			.write()
			.register::<Script<Value, Value>>();
		ScriptConfig::default()
			.assert_writable(
				&ComponentIdent::resolve(
					&mut world,
					<Script<Value, Value>>::short_type_path(),
				)
				.unwrap(),
			)
			.unwrap_err()
			.to_string()
			.xpect_contains("no script may write");
	}

	/// A runtime component that picks a protected name is refused too, which is
	/// the conservative direction: the rule is on the name, not the type.
	#[beet_core::test]
	fn a_runtime_component_cannot_take_a_protected_name() {
		let mut world = test_world();
		DynamicComponents::register(
			&mut world,
			"ScriptConfig",
			ValueSchema::Any,
		)
		.unwrap();
		writable(&ScriptConfig::default(), "ScriptConfig").unwrap_err();
	}
}
