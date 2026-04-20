use crate::prelude::*;
use beet_core::prelude::*;

/// Repeat control-flow component.
///
/// Calls its single child in a loop with a clone of the original input.
/// Returns [`Outcome::Fail`] immediately if the child fails;
/// loops forever otherwise.
/// With no child, returns [`Outcome::Pass`] immediately.
#[derive(Debug, Component)]
#[require(RepeatAction<Input>)]
pub struct Repeat<Input = ()>
where
	Input: 'static + Send + Sync + Clone,
{
	_marker: PhantomData<fn() -> Input>,
}

impl<Input> Clone for Repeat<Input>
where
	Input: 'static + Send + Sync + Clone,
{
	fn clone(&self) -> Self {
		Self {
			_marker: PhantomData,
		}
	}
}
impl<Input> Copy for Repeat<Input> where Input: 'static + Send + Sync + Clone {}

impl<Input> Default for Repeat<Input>
where
	Input: 'static + Send + Sync + Clone,
{
	fn default() -> Self {
		Self {
			_marker: PhantomData,
		}
	}
}

impl Repeat<()> {
	/// Create a default `Repeat<()>`.
	pub fn new() -> Self { Self::default() }
}

#[action(default)]
#[derive(Component)]
pub async fn RepeatAction<Input>(cx: ActionContext<Input>) -> Result<Outcome>
where
	Input: 'static + Send + Sync + Clone,
{
	let child = match cx
		.caller
		.get(|children: &Children| children.first().copied())
		.await
	{
		Ok(Some(child)) => child,
		_ => return Outcome::PASS.xok(),
	};

	let world = cx.world();

	let action_meta = world
		.entity(child)
		.get(|meta: &ActionMeta| *meta)
		.await
		.map_err(|err| {
			bevyhow!("repeat child has no action: {child:?}, error: {err}")
		})?;
	action_meta.assert_match::<Input, Outcome>()?;

	let input = cx.input;
	loop {
		match world
			.entity(child)
			.call::<Input, Outcome>(input.clone())
			.await?
		{
			Outcome::Pass(_) => {}
			Outcome::Fail(_) => return Outcome::FAIL.xok(),
		}
	}
}


/// Repeat-N control-flow component.
///
/// Calls its single child up to `total_times`, passing a clone of the
/// original input each iteration.
/// Returns [`Outcome::Fail`] immediately if the child fails;
/// returns [`Outcome::Pass`] after all iterations complete.
/// With no child, returns [`Outcome::Pass`] immediately.
#[derive(Debug, Component)]
#[require(RepeatTimesAction<Input>)]
pub struct RepeatTimes<Input = ()>
where
	Input: 'static + Send + Sync + Clone,
{
	/// Maximum number of iterations.
	total_times: u32,
	_marker: PhantomData<fn() -> Input>,
}

impl<Input> Clone for RepeatTimes<Input>
where
	Input: 'static + Send + Sync + Clone,
{
	fn clone(&self) -> Self {
		Self {
			total_times: self.total_times,
			_marker: PhantomData,
		}
	}
}
impl<Input> Copy for RepeatTimes<Input> where Input: 'static + Send + Sync + Clone {}

impl<Input> RepeatTimes<Input>
where
	Input: 'static + Send + Sync + Clone,
{
	/// Configured repeat limit.
	pub fn total_times(&self) -> u32 { self.total_times }
}

impl RepeatTimes<()> {
	/// Sentinel used to represent an effectively unbounded repeat count.
	pub const FOREVER: u32 = u32::MAX;

	/// Create a bounded repeat counter.
	pub fn new(total_times: u32) -> Self {
		Self { total_times, _marker: PhantomData }
	}

	/// Create an unbounded repeat counter.
	pub fn forever() -> Self { Self::new(Self::FOREVER) }
}

impl<Input> RepeatTimes<Input>
where
	Input: 'static + Send + Sync + Clone,
{
	/// Create a bounded repeat counter with a typed input marker.
	pub fn typed(total_times: u32) -> Self {
		Self { total_times, _marker: PhantomData }
	}

	/// Create an unbounded typed repeat counter.
	pub fn typed_forever() -> Self { Self::typed(u32::MAX) }
}

/// Action component for [`RepeatTimes`], calls the single child up to
/// `total_times`, returning on first failure.
#[action(default)]
#[derive(Component)]
pub async fn RepeatTimesAction<Input>(cx: ActionContext<Input>) -> Result<Outcome>
where
	Input: 'static + Send + Sync + Clone,
{
	let total_times = cx
		.caller
		.get(|rt: &RepeatTimes<Input>| rt.total_times)
		.await
		.unwrap_or(0);

	let child = match cx
		.caller
		.get(|children: &Children| children.first().copied())
		.await
	{
		Ok(Some(child)) => child,
		_ => return Outcome::PASS.xok(),
	};

	let world = cx.world();

	let action_meta = world
		.entity(child)
		.get(|meta: &ActionMeta| *meta)
		.await
		.map_err(|err| {
			bevyhow!("repeat_times child has no action: {child:?}, error: {err}")
		})?;
	action_meta.assert_match::<Input, Outcome>()?;

	let input = cx.input;
	for _ in 0..total_times {
		match world
			.entity(child)
			.call::<Input, Outcome>(input.clone())
			.await?
		{
			Outcome::Pass(_) => {}
			Outcome::Fail(_) => return Outcome::FAIL.xok(),
		}
	}

	Outcome::PASS.xok()
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::sync::Arc;
	use std::sync::atomic::AtomicU32;
	use std::sync::atomic::Ordering;

	fn outcome_fail() -> Action<(), Outcome> {
		Action::new_pure(|_: ActionContext| Outcome::FAIL.xok())
	}

	/// A child action that passes `n` times, then fails.
	fn pass_n_then_fail(n: u32) -> (Arc<AtomicU32>, Action<(), Outcome>) {
		let count = Arc::new(AtomicU32::new(0));
		let count_inner = count.clone();
		let action = Action::new_pure(move |_: ActionContext| {
			let calls = count_inner.fetch_add(1, Ordering::SeqCst);
			if calls < n {
				Outcome::PASS.xok()
			} else {
				Outcome::FAIL.xok()
			}
		});
		(count, action)
	}

	// ── Repeat ──────────────────────────────────────────────────

	#[beet_core::test]
	async fn repeat_no_child() {
		AsyncPlugin::world()
			.spawn(Repeat::new())
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}

	#[beet_core::test]
	async fn repeat_failing_child() {
		AsyncPlugin::world()
			.spawn((Repeat::new(), children![outcome_fail()]))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::FAIL);
	}

	#[beet_core::test]
	async fn repeat_child_passes_then_fails() {
		let (count, child) = pass_n_then_fail(3);
		AsyncPlugin::world()
			.spawn((Repeat::new(), children![child]))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::FAIL);
		// passed 3 times, failed on 4th call
		count.load(Ordering::SeqCst).xpect_eq(4);
	}

	// ── RepeatTimes ─────────────────────────────────────────────

	#[beet_core::test]
	async fn repeat_times_no_child() {
		AsyncPlugin::world()
			.spawn(RepeatTimes::new(5))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}

	#[beet_core::test]
	async fn repeat_times_all_pass() {
		let (count, child) = pass_n_then_fail(10);
		AsyncPlugin::world()
			.spawn((RepeatTimes::new(3), children![child]))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
		// child was called exactly 3 times
		count.load(Ordering::SeqCst).xpect_eq(3);
	}

	#[beet_core::test]
	async fn repeat_times_child_fails_early() {
		AsyncPlugin::world()
			.spawn((RepeatTimes::new(5), children![outcome_fail()]))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::FAIL);
	}

	#[beet_core::test]
	async fn repeat_times_zero_is_immediate_pass() {
		AsyncPlugin::world()
			.spawn((RepeatTimes::new(0), children![outcome_fail()]))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}

	#[beet_core::test]
	async fn repeat_times_accessors() {
		RepeatTimes::new(7).total_times().xpect_eq(7);
		RepeatTimes::forever()
			.total_times()
			.xpect_eq(RepeatTimes::FOREVER);
	}
}
