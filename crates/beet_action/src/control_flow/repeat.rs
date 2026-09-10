use crate::prelude::*;
use beet_core::prelude::*;

/// Repeat control-flow component.
///
/// Calls its single child in a loop, threading the input as a [`Sequence`]
/// does: each [`Outcome::Pass`] carries the next iteration's input, and the
/// first [`Outcome::Fail`] ends the loop and is returned as the loop's own
/// outcome. The child's fail is the loop's `break`, so whatever it carried (a
/// verdict, a response) reaches the node above unchanged. A `Repeat` never
/// passes: an unbounded loop has no other way out. To bound the iterations use
/// [`RepeatTimes`], which passes once its count completes.
///
/// With no child, returns [`Outcome::Pass`] with the input immediately.
#[action]
#[derive(Debug, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn Repeat<Input = (), Output = ()>(
	cx: ActionContext<Input>,
) -> Result<Outcome<Input, Output>>
where
	Input: 'static + Send + Sync,
	Output: 'static + Send + Sync,
{
	let world = cx.world();
	let Some(child) =
		only_child::<Input, Output>(&world, cx.id(), "repeat").await?
	else {
		return Outcome::Pass(cx.input).xok();
	};
	let mut input = cx.input;
	loop {
		match world
			.entity(child)
			.call::<Input, Outcome<Input, Output>>(input)
			.await?
		{
			Outcome::Pass(next) => input = next,
			Outcome::Fail(output) => return Outcome::Fail(output).xok(),
		}
	}
}

impl Repeat {
	/// Create a default `Repeat<(), ()>`.
	pub fn new() -> Self { Self::default() }
}

/// Repeat-N control-flow component.
///
/// Calls its single child up to `total_times`, threading the input as
/// [`Repeat`] does. Returns the child's first [`Outcome::Fail`] immediately,
/// else [`Outcome::Pass`] with the final input once every iteration completed.
/// With no child, returns [`Outcome::Pass`] with the input immediately.
#[action]
#[derive(Debug, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn RepeatTimes<Input = (), Output = ()>(
	/// Maximum number of iterations.
	#[field]
	total_times: u32,
	cx: ActionContext<Input>,
) -> Result<Outcome<Input, Output>>
where
	Input: 'static + Send + Sync,
	Output: 'static + Send + Sync,
{
	let world = cx.world();
	let Some(child) =
		only_child::<Input, Output>(&world, cx.id(), "repeat_times").await?
	else {
		return Outcome::Pass(cx.input).xok();
	};
	let mut input = cx.input;
	for _ in 0..total_times {
		match world
			.entity(child)
			.call::<Input, Outcome<Input, Output>>(input)
			.await?
		{
			Outcome::Pass(next) => input = next,
			Outcome::Fail(output) => return Outcome::Fail(output).xok(),
		}
	}
	Outcome::Pass(input).xok()
}

impl RepeatTimes {
	/// Sentinel used to represent an effectively unbounded repeat count.
	pub const FOREVER: u32 = u32::MAX;

	/// Create a bounded repeat counter.
	pub fn new(total_times: u32) -> Self { Self::typed(total_times) }

	/// Create an unbounded repeat counter.
	pub fn forever() -> Self { Self::new(Self::FOREVER) }
}

impl<Input, Output> RepeatTimes<Input, Output>
where
	Input: 'static + Send + Sync,
	Output: 'static + Send + Sync,
{
	/// Create a bounded repeat counter with typed input and output.
	pub fn typed(total_times: u32) -> Self {
		Self {
			total_times,
			_marker: PhantomData,
		}
	}

	/// Create an unbounded typed repeat counter.
	pub fn typed_forever() -> Self { Self::typed(u32::MAX) }
}

/// The single child of a loop node, checked to serve
/// `Input -> Outcome<Input, Output>`; [`None`] for a childless node.
async fn only_child<Input, Output>(
	world: &AsyncWorld,
	parent: Entity,
	node: &str,
) -> Result<Option<Entity>>
where
	Input: 'static + Send + Sync,
	Output: 'static + Send + Sync,
{
	let Some(child) = BehaviourChildren::only_for(world, parent).await else {
		return None.xok();
	};
	world
		.entity(child)
		.get(|meta: &ActionMeta| meta.clone())
		.await
		.map_err(|err| {
			bevyhow!("{node} child has no action: {child:?}, error: {err}")
		})?
		.assert_match::<Input, Outcome<Input, Output>>()?;
	Some(child).xok()
}

#[cfg(test)]
mod tests {
	use super::*;
	use alloc::sync::Arc;
	use core::sync::atomic::AtomicU32;
	use core::sync::atomic::Ordering;

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

	/// A child that fails on its first call ends the loop straight away, and the
	/// loop returns that fail.
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

	/// The loop threads each pass into the next call and breaks with the fail
	/// payload, so the node above reads what the body said.
	#[beet_core::test]
	async fn repeat_threads_input_and_breaks_with_output() {
		let body = Action::<u32, Outcome<u32, &'static str>>::new_pure(
			|cx: ActionContext<u32>| match cx.input {
				n if n < 3 => Outcome::Pass(n + 1).xok(),
				_ => Outcome::Fail("done").xok(),
			},
		);
		AsyncPlugin::world()
			.spawn((Repeat::<u32, &'static str>::default(), children![body]))
			.call::<u32, Outcome<u32, &'static str>>(0)
			.await
			.unwrap()
			.xpect_eq(Outcome::Fail("done"));
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
		RepeatTimes::new(7).total_times.xpect_eq(7);
		RepeatTimes::forever()
			.total_times
			.xpect_eq(RepeatTimes::FOREVER);
	}

	/// Fields are read live, so a value edited between calls is observed.
	#[beet_core::test]
	async fn repeat_times_reads_fields_live() {
		let (count, child) = pass_n_then_fail(10);
		let mut world = AsyncPlugin::world();
		let entity = world.spawn((RepeatTimes::new(2), children![child])).id();
		world
			.entity_mut(entity)
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
		count.load(Ordering::SeqCst).xpect_eq(2);
		world
			.entity_mut(entity)
			.get_mut::<RepeatTimes>()
			.unwrap()
			.total_times = 3;
		world
			.entity_mut(entity)
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
		count.load(Ordering::SeqCst).xpect_eq(5);
	}

	/// A detached action has no component to read, so `into_action` freezes the
	/// fields at conversion.
	#[beet_core::test]
	async fn repeat_times_into_action_is_frozen() {
		let (count, child) = pass_n_then_fail(10);
		let mut world = AsyncPlugin::world();
		let entity = world
			.spawn((RepeatTimes::new(2).into_action(), children![child]))
			.id();
		// the component is absent, yet the frozen action still repeats twice
		world
			.entity_mut(entity)
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
		count.load(Ordering::SeqCst).xpect_eq(2);
	}
}
