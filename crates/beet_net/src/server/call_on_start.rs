//! Turning a start into a run: the [`CallOnStart`] verb calls an entity's action
//! when the run it lives under starts.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Start verb: when the run at or above this entity starts, call this entity's
/// action with the start request, detached.
///
/// The [`CallOnReady`] counterpart for the other lifecycle edge. `Ready` is a
/// load, swept over the whole subtree, so [`CallOnReady`] sits on the action it
/// drives and each declares its own. A start is one event on one entity (the
/// server root a [`RunningSet`] parked), so this verb instead rides that
/// [`StartRunning<Request>`] fan-out: a global observer calls every `CallOnStart`
/// at or under the starting entity, and the ancestry filter is what keeps
/// co-resident entries from starting each other's work.
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

	/// Observer: call every [`CallOnStart`] at or under a starting run, once per
	/// start.
	///
	/// Global rather than per-entity so the verb needs no boot machinery of its
	/// own. Each call is detached, so a parked behavior never holds the fan-out
	/// up.
	pub(crate) fn call_on_start(
		ev: On<StartRunning<Request>>,
		children: Query<&Children>,
		verbs: Query<(), With<CallOnStart>>,
		mut commands: Commands,
	) -> Result {
		let parts = ev.with(|request| request.parts().clone())?;
		for entity in children
			.iter_descendants_inclusive(ev.entity)
			.filter(|entity| verbs.contains(*entity))
		{
			let request = Request::from_parts(parts.clone(), default());
			commands
				.entity(entity)
				.queue_async_local(move |entity| Self::call(entity, request));
		}
		Ok(())
	}
}
