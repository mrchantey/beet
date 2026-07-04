//! Cross-platform uuid creation. `uuid`'s own v7 clock is std-only (it panics
//! `time not implemented` on wasm32-unknown-unknown), so time-based ids derive
//! their timestamp from [`time_ext`], which has wasm and no_std backends.
use beet_core::prelude::*;
use std::sync::LazyLock;
use std::sync::Mutex;
use uuid::Uuid;
use uuid::timestamp::context::ContextV7;

/// The shared v7 counter context, so ids created in the same tick still sort
/// in creation order (matching `Uuid::now_v7`'s own process-wide guarantee).
/// Mutexed: [`ContextV7`] is interior-mutable and not `Sync`.
static CONTEXT: LazyLock<Mutex<ContextV7>> =
	LazyLock::new(|| Mutex::new(ContextV7::new()));

/// A [`Uuid::now_v7`] that works on every target, deriving its timestamp from
/// [`time_ext::now`] instead of the std-only system clock. Monotonic within
/// the process, so rapid successive ids sort in creation order.
pub fn now_v7() -> Uuid {
	let now = time_ext::now();
	let context = CONTEXT.lock().unwrap();
	Uuid::new_v7(uuid::Timestamp::from_unix(
		&*context,
		now.as_secs(),
		now.subsec_nanos(),
	))
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Runs on wasm too, where `Uuid::now_v7` itself panics.
	#[beet_core::test]
	fn embeds_the_wall_clock() {
		let id = uuid_ext::now_v7();
		id.get_version_num().xpect_eq(7);
		// the embedded timestamp matches the clock it derives from
		let (secs, _nanos) = id.get_timestamp().unwrap().to_unix();
		let now_secs = time_ext::now().as_secs();
		(now_secs.abs_diff(secs) <= 1).xpect_true();
	}

	/// The shared counter context keeps same-tick ids in creation order.
	#[beet_core::test]
	fn ids_are_monotonic() {
		let ids = (0..64).map(|_| uuid_ext::now_v7()).collect::<Vec<_>>();
		let mut sorted = ids.clone();
		sorted.sort();
		ids.xpect_eq(sorted);
	}
}
