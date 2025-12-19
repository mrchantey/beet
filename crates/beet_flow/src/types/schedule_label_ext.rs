use crate::prelude::*;
use beet_core::prelude::*;




#[extend::ext(name=BeetFlowScheduleLabelExt)]
pub impl<T> T
where
	T: Default + ScheduleLabel,
{
	/// Runs the schedule then triggers an [`Outcome::Pass`]
	fn action() -> impl Bundle {
		(
			Name::new(format!("Run Schedule - {:?}", T::default())),
			OnSpawn::observe(
				|mut ev: On<GetOutcome>, mut commands: Commands| {
					commands.run_system_cached(T::run());
					ev.trigger_with_cx(Outcome::Pass);
				},
			),
		)
	}
}
