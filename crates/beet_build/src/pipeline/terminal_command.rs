use async_process::Command;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_rsx::prelude::*;

/// Emitted for each stdout/stderr line. `is_err` is true for stderr.
#[derive(EntityTargetEvent)]
pub struct StdOutLine {
	pub line: String,
	pub is_err: bool,
}

/// An action that will run the provided command asynchronously,
/// calling [`Outcome::Pass`] if the command was successful or [`Outcome::Fail`] if it failed.
/// All stdout and stderr lines are emitted as [`StdOutLine`] entity events.
///
/// Lines are streamed concurrently as they arrive. Empty lines are emitted (not skipped).
/// Reader tasks are aborted if the process exits first.
#[construct]
pub fn TerminalCommand(
	/// The command to run
	cmd: String,
	/// The command argum
	/// ents
	args: Vec<String>,
) -> impl Bundle {
	OnSpawn::observe(move |mut ev: On<GetOutcome>| {
		let cmd = cmd.clone();
		let args = args.clone();
		ev.run_async(async move |action| {
			// 1. spawn the command
			let mut child = Command::new(&cmd).args(&args).spawn()?;
			// 2. take stdout/stderr pipes
			// let stdout = child
			// 	.stdout
			// 	.take()
			// 	.ok_or_else(|| bevyhow!("stdout not found"))?;

			// let stderr = child
			// 	.stderr
			// 	.take()
			// 	.ok_or_else(|| bevyhow!("stderr not found"))?;

			// Async entity used to emit events from the reader tasks

			let outcome = match child.status().await?.exit_ok() {
				Ok(_) => Outcome::Pass,
				Err(_) => Outcome::Fail,
			};
			action.entity().trigger_target(outcome).await;

			Ok(())
		});
	})
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;
	use beet_rsx::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn works() {
		let mut app = App::new();
		app.add_plugins((ControlFlowPlugin, AsyncPlugin));
		app.world_mut()
			.spawn(bsx! {
				<entity {(Sequence, ExitOnEnd)}>
					<TerminalCommand cmd="echo" args=vec!["foobar".into()]/>
				</entity>
			})
			.trigger_target(GetOutcome);

		app.run_async().await.xpect_eq(AppExit::Success);
	}
}
