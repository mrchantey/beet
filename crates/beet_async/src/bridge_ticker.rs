//! Per-frame executor ticking for the sync-point driver.
//!
//! The driver has to give woken futures a chance to poll while `&mut World` is
//! published. Natively that is bevy's global task pools. On wasm bevy's
//! `spawn_local` routes futures to the JS event loop, which a synchronous
//! driver cannot tick, so the consumer (beet) spawns bridged tasks onto its own
//! tickable executor and registers the seam below to reach it.
//!
//! ## Why the hook resolves per world
//!
//! Ticking is re-entrant by design: a ticked task may drive another app, whose
//! sync point ticks again so *its* bridged requests can poll while *its* world
//! is published. That nesting is structural and shallow.
//!
//! Resolving the executor from the world being driven is what keeps it that
//! way. One executor shared across every world would have each nesting level
//! also poll whatever unrelated tasks happen to be runnable, so the call stack
//! would grow with the number of in-flight tasks anywhere in the process rather
//! than with the nesting: a loaded test binary (one world per test) would reach
//! any depth cap on load alone, and sync points would start being deferred as a
//! throttle rather than as a backstop.

use bevy::ecs::world::World;
#[cfg(target_arch = "wasm32")]
use bevy::platform::sync::Arc;
#[cfg(target_arch = "wasm32")]
use bevy::platform::sync::OnceLock;

/// The executor serving one world, ready to tick.
///
/// wasm only: natively the driver ticks bevy's global task pools, which need no
/// resolving.
#[cfg(target_arch = "wasm32")]
#[derive(Clone)]
pub struct WorldTicker(Arc<dyn Fn() + Send + Sync>);

#[cfg(target_arch = "wasm32")]
impl WorldTicker {
	/// Wrap the closure that polls one world's runnable tasks.
	pub fn new(tick: Arc<dyn Fn() + Send + Sync>) -> Self { Self(tick) }
}

#[cfg(target_arch = "wasm32")]
static RESOLVE: OnceLock<fn(&World) -> Option<WorldTicker>> = OnceLock::new();

/// The consumer-registered seam resolving a world to the executor serving it.
///
/// Registered by beet's own `AsyncPlugin`, which owns the executor; this crate
/// only knows that a world may have one.
#[cfg(target_arch = "wasm32")]
pub struct WorldTickerHook;

#[cfg(target_arch = "wasm32")]
impl WorldTickerHook {
	/// Register the resolver. Only the first registration is kept: this reaches
	/// the consumer crate, and there is one of those.
	pub fn set(resolve: fn(&World) -> Option<WorldTicker>) {
		let _ = RESOLVE.set(resolve);
	}
}

/// The ticker for the world a sync point is driving, resolved once per driver
/// run and reused for every tick within it.
pub(crate) struct BridgeTicker {
	#[cfg(target_arch = "wasm32")]
	ticker: Option<WorldTicker>,
}

impl BridgeTicker {
	/// Resolve the ticker for `world`.
	pub(crate) fn resolve(world: &World) -> Self {
		#[cfg(not(target_arch = "wasm32"))]
		let _ = world;
		Self {
			#[cfg(target_arch = "wasm32")]
			ticker: RESOLVE.get().and_then(|resolve| resolve(world)),
		}
	}

	/// Give this world's woken futures a chance to poll.
	pub(crate) fn tick(&self) {
		#[cfg(not(target_arch = "wasm32"))]
		bevy::tasks::tick_global_task_pools_on_main_thread();
		#[cfg(target_arch = "wasm32")]
		if let Some(ticker) = &self.ticker {
			(ticker.0)();
		}
	}
}
