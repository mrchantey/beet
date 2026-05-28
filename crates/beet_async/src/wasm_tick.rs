//! Wasm bridge ticking hook.
//!
//! On wasm, bevy's `spawn_local` routes futures to the JS event loop, which the
//! synchronous sync-point driver cannot tick. So the consumer (beet) spawns
//! bridged tasks onto its own tickable executor and registers a hook here; the
//! driver calls it to poll those futures while `&mut World` is published.

use bevy::platform::sync::OnceLock;

static TICK_HOOK: OnceLock<fn()> = OnceLock::new();

/// Register the executor-ticking hook used by the sync-point driver on wasm.
///
/// Only the first registration is kept (the bridge runs single-threaded on wasm,
/// so one shared executor suffices).
pub fn set_wasm_tick_hook(hook: fn()) { let _ = TICK_HOOK.set(hook); }

/// Tick the registered executor, if any. Called by the driver in place of
/// `tick_global_task_pools_on_main_thread` (which is absent on wasm).
pub(crate) fn tick() {
	if let Some(hook) = TICK_HOOK.get() {
		hook();
	}
}
