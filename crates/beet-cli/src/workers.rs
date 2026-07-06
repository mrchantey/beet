//! The Cloudflare Worker runner seam: a [`WorkersPlugin`] that makes the
//! per-isolate [`App`] Worker-shaped, plus the per-isolate [`World`] cell the
//! `#[event(fetch)]` handler drives.
//!
//! A Worker builds the *same* [`App`] the native binary does (`BeetPlugins` via
//! [`build_app`]); `WorkersPlugin` is the only divergence. It does two things:
//!
//! 1. Installs a no-op runner via [`App::set_runner`]. `BeetPlugins` adds the
//!    headless [`ScheduleRunnerPlugin`](bevy::app::ScheduleRunnerPlugin) whose
//!    runner *busy-spins* the schedule loop, which would peg (and block) the
//!    Worker's single JS thread. The Worker never drives the world through a
//!    schedule loop, it drives it per-fetch through `exchange`, so the runner is
//!    a no-op that returns [`AppExit::Success`] without looping. Last
//!    `set_runner` wins, so this must be added *after* `BeetPlugins`.
//! 2. Owns the per-isolate [`World`] cell ([`WorkerWorld`]) the fetch handler
//!    takes the world out of for the duration of an exchange and puts back after.
//!    [`World`] is `!Send` on wasm, so the cell is a `thread_local` `RefCell`
//!    (the Worker runtime is single-threaded, so the isolate-global slot is a
//!    thread-local), not an `OnceLock`.
//!
//! The build itself stays *lazy on first fetch*: the Worker runtime forbids
//! blocking on the JS thread, so the runner cannot `block_on` the async entry
//! build. The handler instead settles the build to readiness
//! ([`settle_until_ready`]) the first time it takes the world.

use crate::prelude::*;
use beet::prelude::*;
use std::cell::RefCell;

/// Build the per-isolate [`App`] a Worker drives: the same [`BeetPlugins`] the
/// native binary builds (on wasm this resolves to the headless runner + the render
/// router stack), plus [`WorkersPlugin`] for the Worker-appropriate runner.
///
/// Shares its plugin construction with the native binary's `main` (which builds
/// `BeetPlugins` + a `Startup` `load_entry`); the Worker swaps that `Startup`
/// build/run for a lazy per-fetch build, so it adds [`WorkersPlugin`] instead.
pub fn build_app() -> App {
	let mut app = App::new();
	app.add_plugins(BeetPlugins).add_plugins(WorkersPlugin);
	// the binary's compiled surface, so a loaded site's `<CrateCheck/>` verifies
	// against this Worker build like any other entry driver.
	app.world_mut().spawn(cli_registration());
	app
}

/// Makes a per-isolate [`App`] Worker-shaped: installs a no-op runner (the Worker
/// drives the world per-fetch, never through the blocking schedule loop) and owns
/// the per-isolate [`WorkerWorld`] cell. See the [module docs](self).
#[derive(Default)]
pub struct WorkersPlugin;

impl Plugin for WorkersPlugin {
	fn build(&self, app: &mut App) {
		// the Worker serves each request by driving the world through `exchange`,
		// so the schedule loop must never run: a no-op runner that exits at once
		// (instead of `ScheduleRunnerPlugin`'s busy-spin, which would block the JS
		// thread). Added after `BeetPlugins`, so this is the runner that wins.
		app.set_runner(|_app| AppExit::Success);
	}
}

thread_local! {
	/// The per-isolate built [`World`], reused across requests. Taken out for the
	/// duration of an exchange (so the exchange can borrow the world mutably across
	/// an await) and put back after.
	///
	/// `thread_local` not `OnceLock`: [`World`] is `!Send` on wasm. The Worker
	/// runtime is single-threaded, so a thread-local is the isolate-global slot.
	static WORLD: RefCell<Option<WorkerWorld>> = const { RefCell::new(None) };
}

/// The per-isolate built [`World`] plus the entry version it was built from, so a
/// re-synced bucket rebuilds the world on the next request.
pub struct WorkerWorld {
	/// The built site world, driven per-fetch.
	pub world: World,
	/// The host entity carrying the [`Router`] action exchanges dispatch to.
	pub host: Entity,
	/// The store object version of the entry document at build time, or `None` if
	/// the head check was unavailable (then every request rebuilds).
	pub version: Option<String>,
}

impl WorkerWorld {
	/// Take the per-isolate world out of the cell, if built. The caller must
	/// [`put`](Self::put) it back (or rebuild) after the exchange.
	pub fn take() -> Option<Self> {
		WORLD.with(|slot| slot.borrow_mut().take())
	}

	/// Put a built world back into the per-isolate cell for the next request.
	pub fn put(self) { WORLD.with(|slot| *slot.borrow_mut() = Some(self)); }
}
