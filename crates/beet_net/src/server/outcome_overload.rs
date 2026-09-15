//! Serving a `() -> Outcome` behaviour through the request shapes.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Serves a `() -> Outcome` behaviour through the request shapes, so a
/// behaviour-tree leaf is a load root or a route step with no adapter of its
/// own: the `Request -> Response` a load or a route dispatches, and the
/// `Request -> Outcome<Request, Response>` a sequence route threads.
///
/// ```bsx
/// <Sequence {(CallOnReady, OutcomeOverload)}>
///     <CheckOne/>
///     <CheckTwo/>
/// </Sequence>
/// <Route path="dump" {ExchangeSequence}>
///     <Command exe="deno" args={["run", "dump.ts"]} {OutcomeOverload}/>
/// </Route>
/// ```
///
/// An [`Outcome`] is a branch, not an error: a `Fail` at a `<Sequence>` root
/// means a step did not pass, at a `<Repeat>` root that the loop ended. So a
/// `Fail` answers `200` like a `Pass`, still ending any sequence it is a step
/// of, and only an action error is an error. A scene whose outcome *is* its
/// status opts in with `error_on_fail`, which raises a `Fail` as an error: a
/// `500`, and at a load root a nonzero exit.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(
	ActionOverload<Request, Response> = load_overload(),
	ActionOverload<Request, Outcome<Request, Response>> = step_overload(),
)]
pub struct OutcomeOverload {
	/// Raise a `Fail` as an error rather than answering `200`.
	pub error_on_fail: bool,
}

impl OutcomeOverload {
	/// The opt-in making the outcome the status: a `Fail` is an error.
	pub fn error_on_fail() -> Self {
		Self {
			error_on_fail: true,
		}
	}
}

/// Serves the load and dispatch pair: `Pass` and `Fail` both answer `200`.
fn load_overload() -> ActionOverload<Request, Response> {
	ActionOverload::new(Action::new_async(
		async |cx: ActionContext<Request>| -> Result<Response> {
			call_leaf(&cx.caller).await?;
			Response::ok().xok()
		},
	))
}

/// Serves the sequence step pair: `Pass` threads the request on, `Fail` ends
/// the sequence with a `200`.
fn step_overload() -> ActionOverload<Request, Outcome<Request, Response>> {
	ActionOverload::new(Action::new_async(
		async |cx: ActionContext<Request>| -> Result<Outcome<Request, Response>> {
			match call_leaf(&cx.caller).await? {
				Pass(()) => Pass(cx.input),
				Fail(()) => Fail(Response::ok()),
			}
			.xok()
		},
	))
}

/// Call the entity's canonical `() -> Outcome` leaf directly rather than
/// re-entering resolution, so neither overload can recurse, raising a `Fail`
/// as an error under `error_on_fail`.
async fn call_leaf(caller: &AsyncEntity) -> Result<Outcome> {
	let leaf = caller
		.get(|action: &Action<(), Outcome>| action.clone())
		.await?;
	let error_on_fail = caller
		.get(|this: &OutcomeOverload| this.error_on_fail)
		.await?;
	match caller.call_detached(leaf, ()).await? {
		Fail(()) if error_on_fail => {
			bevybail!("behaviour {} failed", caller.id())
		}
		outcome => outcome.xok(),
	}
}

#[cfg(test)]
mod test {
	use super::*;

	fn leaf(outcome: Outcome) -> Action<(), Outcome> {
		Action::new_pure(move |_: ActionContext| outcome.xok())
	}

	/// The load pair: a marked leaf called as `Request -> Response`.
	async fn load(bundle: impl Bundle) -> Result<Response> {
		AsyncPlugin::world()
			.spawn(bundle)
			.call::<Request, Response>(Request::get("/"))
			.await
	}

	/// The step pair: a marked leaf as the one step of a request-threading
	/// sequence.
	async fn step(bundle: impl Bundle) -> Result<Outcome<Request, Response>> {
		AsyncPlugin::world()
			.spawn((Sequence::<Request, Response>::default(), children![
				bundle
			]))
			.call::<Request, Outcome<Request, Response>>(Request::get("/"))
			.await
	}

	#[beet_core::test]
	async fn load_answers_ok_whatever_the_outcome() {
		for outcome in [Outcome::PASS, Outcome::FAIL] {
			load((OutcomeOverload::default(), leaf(outcome)))
				.await
				.unwrap()
				.status()
				.xpect_eq(StatusCode::OK);
		}
	}

	#[beet_core::test]
	async fn load_fail_errors_when_opted_in() {
		load((OutcomeOverload::error_on_fail(), leaf(Outcome::FAIL)))
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("failed");
	}

	#[beet_core::test]
	async fn step_pass_threads_the_request() {
		step((OutcomeOverload::default(), leaf(Outcome::PASS)))
			.await
			.unwrap()
			.xmap(|outcome| matches!(outcome, Pass(_)))
			.xpect_true();
	}

	#[beet_core::test]
	async fn step_fail_ends_the_sequence_ok() {
		let Fail(response) =
			step((OutcomeOverload::default(), leaf(Outcome::FAIL)))
				.await
				.unwrap()
		else {
			panic!("a failing step must end the sequence");
		};
		response.status().xpect_eq(StatusCode::OK);
	}

	#[beet_core::test]
	async fn step_fail_errors_when_opted_in() {
		step((OutcomeOverload::error_on_fail(), leaf(Outcome::FAIL)))
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("failed");
	}
}
