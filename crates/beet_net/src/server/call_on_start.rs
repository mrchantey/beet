//! Turning a start into a run: the [`CallOnStart`] verb calls an entity's action
//! when the run it lives under starts.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Start verb: when a swept run start reaches this entity, call this entity's
/// action with the start request, detached.
///
/// The [`CallOnReady`] counterpart for the other lifecycle edge, and the same
/// shape: `Ready` sweeps a loaded subtree, and a run root declaring
/// [`SweepDescendants`] sweeps its [`StartRunning<Request>`] over the started
/// one, so each verb sits on the action it drives and observes its own entity.
/// The sweep never leaves its root's subtree, which is what keeps co-resident
/// entries from starting each other's work. The declaration is deliberate: a
/// root without it fires its start on itself alone, and this verb never hears
/// it.
///
/// ```bsx
/// <CallOnReady {(TuiServer, HttpServer, SweepDescendants)}>  // the run
///     <Router>
///         <Route path="/">..</Route>            // what it serves
///         <Repeat {CallOnStart}>..</Repeat>     // what it drives alongside
///     </Router>
/// </CallOnReady>
/// ```
///
/// Nothing awaits the call: it is made with a handler of its own
/// ([`OutHandler::default`]), so it is one of the entity's [`ActionCalls`]
/// and no task holds it. A finite behavior completes while the servers keep
/// serving, an endless one simply never returns, and only a despawn or an
/// [`InterruptRun`] above it stops it. It owns no action slot and never writes
/// [`AppExit`]: process lifetime belongs to the run, not to what it started.
///
/// The call resolves whichever shape the entity serves:
///
/// - `Request -> Outcome`, for a behavior whose start depends on the request
///   (a thread reading `--new`)
/// - `() -> Outcome`, a plain behavior
///
/// The outcome is the behavior's own; nothing above a start consumes it, so
/// only an error propagates, through the world's error handler.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = hook_ext::observe(on_start_call))]
pub struct CallOnStart;

/// On the start sweep reaching this entity, call its action with the start
/// request, so a parked behavior never holds the sweep up.
fn on_start_call(
	ev: On<StartRunning<Request>>,
	metas: Query<&ActionMeta>,
	mut commands: Commands,
) -> Result {
	let parts = ev.with(|request| request.parts().clone())?;
	let mut entity = commands.entity(ev.entity);
	match metas
		.get(ev.entity)
		.is_ok_and(|meta| meta.matches::<Request, Outcome>())
	{
		true => entity.call::<Request, Outcome>(
			Request::from_parts(parts, default()),
			OutHandler::default(),
		),
		false => entity.call::<(), Outcome>((), OutHandler::default()),
	}
	Ok(())
}
