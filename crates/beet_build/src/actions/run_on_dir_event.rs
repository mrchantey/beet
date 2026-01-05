use beet_core::prelude::*;
use beet_flow::prelude::*;

/// Listens for file watch events and retriggers the associated action,
/// cancelling any children with [`Running`], [`ChildHandle`] etc.
#[action(run_on_dir_event)]
#[derive(Component)]
// TODO FsWatcher cancel listener on remove, so this
// could be replaced without duplicating listeners
// #[require(FsWatcher)]
pub struct RunOnDirEvent;

// we dont care about which events, just retrigger th
fn run_on_dir_event(ev: On<DirEvent>, mut commands: Commands) {
	commands.entity(ev.target()).trigger_target(GetOutcome);
}

#[cfg(test)]
mod tests {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;

	#[sweet::test]
	async fn works() {
		let tempdir = TempDir::new().unwrap();

		let mut app = App::new();
		app.add_plugins(CliPlugin)
			.world_mut()
			.spawn((
				// FsWatcher::default(),
				FsWatcher::new(tempdir.path().clone()),
				RunOnDirEvent,
				Sequence,
				ExitOnEnd,
				children![(
					ContinueRun,
					// sleep at least 100 millis
					CommandConfig::parse_shell("sleep 0.1 && false")
						.into_action()
				)],
			))
			.trigger_target(GetOutcome);

		let tempdir = tempdir.path().clone();
		app.world_mut().run_async(async move |world| {
			time_ext::sleep_millis(10).await;
			// cancels the failing command
			fs_ext::write(tempdir.join("foobar.txt"), "foobar").unwrap();
			// wait enough time for the first command to fail
			// if not cancelled
			time_ext::sleep_millis(60).await;
			world.write_message(AppExit::Success);
		});
		app.run_async().await.xpect_eq(AppExit::Success);
	}
}
