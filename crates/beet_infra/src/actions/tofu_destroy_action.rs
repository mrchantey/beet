//! Tofu destroy step for teardown groups.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// The teardown step that removes the stack's terraform-owned resources: it
/// renders the stack exactly as [`TofuApply`] does, then runs `tofu destroy`.
///
/// It removes nothing else. The state object, the native S3 lock, the artifacts
/// bucket and the work directory are [`StackTeardown`]'s, and they converge
/// before the apply, so a destroy group authored in convergence order runs them
/// after this one.
///
/// A stack must always be tearable down, so the render resolves each content
/// variable if it can and falls back to empty if it cannot: a teardown blocked
/// because a parameter it was about to orphan had already been deleted would be
/// a trap rather than a guard.
#[action]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn TofuDestroy(
	/// Destroy lock-free, clearing any lock an interrupted run left behind.
	///
	/// The recovery path for state nothing holds but a stale lock. `--force` on
	/// the destroy route sets it too, with one edge worth knowing: that mode
	/// also continues past a failing step, and a step that fails takes the
	/// request with it, so a `--force` run whose *earlier* steps failed reaches
	/// this one without the flag. Declaring `force=true` here is how a stack
	/// that is only ever torn down this way says so once, in the document.
	#[field]
	force: bool,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let force = force || cx.input.has_param("force");
	let result = terra::Project::resolve(&cx.caller)
		.await?
		.tofu_destroy(force)
		.await?;
	trace!("{result}");
	Pass(cx.input).xok()
}
