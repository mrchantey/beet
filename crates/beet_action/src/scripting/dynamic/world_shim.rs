//! The `world` global, one source shared by every backend.

/// The `world` API a world-bridged script sees, installed before its source
/// runs.
///
/// Where the two `console` shims mirror each other by convention, this one is
/// literally the same string on every backend: it names one host hook per
/// direction, and each host installs its half the way it can. So there is no
/// per-backend contract to drift.
///
/// - `__world_send(call)` — the host's inbound hook, taking one [`WorldCall`]
///   as a plain object. The embedded engine binds a host function; a host-realm
///   runner wraps it in a `ScriptEvent`.
/// - `__world_reply(reply)` — the shim's own outbound hook, which the host
///   calls with one [`WorldReply`] to settle the promise its `id` names.
///
/// Every `world` method returns a promise the reply settles, so an `await` is a
/// real operation against the live world at the moment it runs: a script reads
/// its own writes, and a refused call rejects at the call site.
///
/// `world.entity(id)` narrows the same API to one entity, the JS face of an
/// [`AsyncEntity`]: `get`/`insert`/`remove`/`despawn` with the entity already
/// supplied, plus the `get_field`/`set_field`/`with_field` document helpers. An
/// event script is handed its own event target as `target`.
///
/// `with_field(path, func)` is the read-apply-write pair composed in the shim,
/// since a closure has no wire form and so could never be a host operation. It
/// is why `target.with_field('count', count => count + 1)` is the whole counter:
/// the read that would otherwise force an `await` happens inside the helper.
///
/// [`AsyncEntity`]: beet_core::prelude::AsyncEntity
///
/// [`WorldCall`]: crate::prelude::WorldCall
/// [`WorldReply`]: crate::prelude::WorldReply
pub(crate) const WORLD_SHIM: &str = include_str!("world_shim.js");

#[cfg(test)]
mod test {
	use super::WORLD_SHIM;
	use beet_core::prelude::*;

	/// The shim names exactly the two hooks every host promises to honour, one
	/// per direction.
	#[beet_core::test]
	fn the_shim_names_its_host_hooks() {
		WORLD_SHIM.xpect_contains("__world_send");
		WORLD_SHIM.xpect_contains("__world_reply");
	}

	/// The entity handle offers every operation an entity can be the subject
	/// of, so `world.entity(id)` and `target` are never a narrower API than the
	/// `world` calls they stand in for.
	#[beet_core::test]
	fn the_handle_offers_every_entity_operation() {
		for method in [
			"get",
			"insert",
			"remove",
			"despawn",
			"get_field",
			"set_field",
			"with_field",
		] {
			WORLD_SHIM.xpect_contains(&format!("{method}: ("));
		}
	}

	/// Every operation the `world` API offers is present in the one shared
	/// source, so no backend can be missing one.
	#[beet_core::test]
	fn the_shim_offers_every_operation() {
		for op in [
			"get",
			"entities",
			"schema",
			"spawn",
			"insert",
			"remove",
			"despawn",
			"get_field",
			"set_field",
		] {
			WORLD_SHIM.xpect_contains(&format!("op: \"{op}\""));
		}
	}
}
