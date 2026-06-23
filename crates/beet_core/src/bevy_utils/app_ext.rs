//! Free-function utilities for [`App`] that are too niche to live on the
//! [`BeetCoreAppExt`](crate::prelude::BeetCoreAppExt) extension trait.

use crate::prelude::*;
use bevy::prelude::*;

/// Drives `app` frame-by-frame until `cond` holds (checked before each update),
/// ticking the async runtime between frames, up to a safety cap. Returns whether
/// the condition was met.
///
/// Unlike [`App::update_async`](crate::prelude::BeetCoreAppExt::update_async),
/// which settles to idle, this stops at a concrete world condition, so it drives
/// a *parked* task that never goes idle (eg an http/socket server holding its
/// accept loop open) to an observable state. The cap fails fast on a never-met
/// condition rather than hanging, notably on the wasm event loop where settling
/// to idle would burn the frame budget. See [`AsyncRunner::tick`].
pub async fn update_until(
	app: &mut App,
	mut cond: impl FnMut(&mut World) -> bool,
) -> bool {
	// guard against a condition that never holds (eg a misbehaving server)
	const MAX_FRAMES: usize = 10_000;
	for _ in 0..MAX_FRAMES {
		if cond(app.world_mut()) {
			return true;
		}
		app.update();
		AsyncRunner::tick().await;
	}
	cond(app.world_mut())
}
