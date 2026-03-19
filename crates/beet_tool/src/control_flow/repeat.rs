use crate::prelude::*;
use beet_core::prelude::*;

/// Repeats indefinitely.
#[derive(Debug, Default, Clone, Copy, Component)]
#[require(
	RepeatTimes = RepeatTimes::forever(),
	Tool<(), Outcome> = async_tool(repeat_tool)
)]
pub struct Repeat;

/// Repeats up to `total_times`.
#[derive(Debug, Clone, Copy, Component)]
#[require(Tool<(), Outcome> = async_tool(repeat_tool))]
pub struct RepeatTimes {
	/// Number of successful repeat passes already emitted.
	num_times: u32,
	/// Maximum number of successful repeat passes to emit.
	total_times: u32,
}

impl RepeatTimes {
	/// Sentinel used to represent an effectively unbounded repeat count.
	pub const FOREVER: u32 = u32::MAX;

	/// Create a bounded repeat counter.
	pub fn new(total_times: u32) -> Self {
		Self {
			num_times: 0,
			total_times,
		}
	}

	/// Create an unbounded repeat counter.
	pub fn forever() -> Self { Self::new(Self::FOREVER) }

	/// Number of successful repeat passes already emitted.
	pub fn num_times(&self) -> u32 { self.num_times }

	/// Configured repeat limit.
	pub fn total_times(&self) -> u32 { self.total_times }
}

/// Emits [`Outcome::PASS`] up to `total_times`, then [`Outcome::FAIL`].
///
/// For [`Repeat`] this is effectively unbounded by default (`u32::MAX`).
pub async fn repeat_tool(cx: AsyncToolIn<()>) -> Result<Outcome> {
	let should_pass = cx
		.caller
		.get_mut(|mut repeat_times: Mut<RepeatTimes>| {
			if repeat_times.num_times < repeat_times.total_times {
				repeat_times.num_times += 1;
				true
			} else {
				false
			}
		})
		.await
		.unwrap_or(false);

	if should_pass {
		Outcome::PASS.xok()
	} else {
		Outcome::FAIL.xok()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[beet_core::test]
	async fn repeat_defaults_to_forever_behavior() {
		let mut world = AsyncPlugin::world();
		let entity = world.spawn(Repeat).id();

		world
			.entity_mut(entity)
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);

		world
			.entity_mut(entity)
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);

		world
			.entity_mut(entity)
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}

	#[beet_core::test]
	async fn repeat_times_limits_passes() {
		let mut world = AsyncPlugin::world();
		let entity = world.spawn(RepeatTimes::new(2)).id();

		world
			.entity_mut(entity)
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);

		world
			.entity_mut(entity)
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);

		world
			.entity_mut(entity)
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::FAIL);
	}

	#[beet_core::test]
	async fn repeat_times_new_starts_at_zero() {
		RepeatTimes::new(7).num_times().xpect_eq(0);
		RepeatTimes::new(7).total_times().xpect_eq(7);
	}
}
