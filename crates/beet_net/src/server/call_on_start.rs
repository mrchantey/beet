//! Turning a start into a run: the [`CallOnStart`] verb calls an entity's action
//! when the run it lives under starts.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Start verb: when the run at or above this entity starts, call this entity's
/// action with the start request, detached.
///
/// The [`CallOnReady`] counterpart for the other lifecycle edge, and the same
/// shape: `Ready` sweeps a loaded subtree, and a [`RunningSet`]'s
/// [`StartRunning<Request>`] sweeps the started one (its required
/// [`StartDescendants`] scopes the [`ScopedTrigger`]), so each verb sits on the
/// action it drives and observes its own entity. The sweep never leaves its
/// root's subtree, which is what keeps co-resident entries from starting each
/// other's work.
///
/// ```bsx
/// <CallOnReady {(TuiServer, HttpServer)}>       // the run
///     <Router>
///         <Route path="/">..</Route>            // what it serves
///         <Repeat {CallOnStart}>..</Repeat>     // what it drives alongside
///     </Router>
/// </CallOnReady>
/// ```
///
/// The call is detached, so a finite behavior completes while the servers keep
/// serving, and an endless one simply never returns. It owns no action slot and
/// never writes [`AppExit`]: process lifetime belongs to the run, not to what it
/// started.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = hook_ext::observe(on_start_call))]
pub struct CallOnStart;

impl CallOnStart {
	/// Call `entity`'s action with `request`, resolving whichever shape it serves:
	///
	/// - `Request -> Outcome`, for a behavior whose start depends on the request
	///   (a thread reading `--new`)
	/// - `() -> Outcome`, a plain behavior
	///
	/// The outcome is the behavior's own; nothing above a start consumes it, so
	/// only an error propagates.
	pub async fn call(entity: AsyncEntity, request: Request) -> Result {
		match entity
			.get(|meta: &ActionMeta| meta.matches::<Request, Outcome>())
			.await
			.unwrap_or(false)
		{
			true => entity.call::<Request, Outcome>(request).await?,
			false => entity.call::<(), Outcome>(()).await?,
		};
		Ok(())
	}
}

/// On the start sweep reaching this entity, queue [`CallOnStart::call`] with the
/// start request, detached, so a parked behavior never holds the sweep up.
fn on_start_call(
	ev: On<StartRunning<Request>>,
	mut commands: Commands,
) -> Result {
	let parts = ev.with(|request| request.parts().clone())?;
	let request = Request::from_parts(parts, default());
	commands
		.entity(ev.entity)
		.queue_async_local(move |entity| CallOnStart::call(entity, request));
	Ok(())
}
