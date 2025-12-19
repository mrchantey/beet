use beet_core::prelude::*;
use beet_flow::prelude::*;



/// Listens for file watch events and retriggers the associated action,
/// cancelling any children with [`Running`], [`ChildHandle`] etc.
#[action(process_watch_events)]
#[derive(Component)]
pub struct DirEventLoop;

// we dont care about which events, just retrigger
pub fn process_watch_events(ev: On<DirEvent>, mut commands: Commands) {
	commands.entity(ev.target()).trigger_target(GetOutcome);
}
