//! Extension trait for creating actions from schedule labels.
use crate::prelude::*;
use beet_core::prelude::*;




/// Extension trait for [`ScheduleLabel`] types to create action bundles.
///
/// This trait adds a method to schedule labels that creates an action bundle
/// which runs the schedule when triggered with [`GetOutcome`].
#[extend::ext(name=BeetFlowScheduleLabelExt)]
pub impl<T> T
where
	T: Default + ScheduleLabel,
{
	/// Creates an action bundle that runs this schedule when triggered.
	///
	/// The action triggers [`Outcome::Pass`] immediately after running the schedule.
	///
	/// # Example
	///
	/// ```ignore
	/// # use beet_core::prelude::*;
	/// # use beet_flow::prelude::*;
	/// #[derive(Default, ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
	/// struct MySchedule;
	///
	/// let mut world = World::new();
	/// world.spawn(MySchedule::action());
	/// ```
	fn action() -> impl Bundle {
		(
			Name::new(format!("Run Schedule - {:?}", T::default())),
			OnSpawn::observe(|ev: On<GetOutcome>, mut commands: Commands| {
				commands.run_system_cached(T::as_system());
				commands.entity(ev.target()).trigger_target(Outcome::Pass);
			}),
		)
	}
}
