//! Cargo build step for deploy sequences.
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Cargo build step for deploy exchange sequences.
/// Reads [`CargoBuildCmd`] from self or an ancestor and executes the build.
/// Returns `Pass` on success to continue the sequence.
#[action]
#[derive(Default, Component)]
pub async fn CargoBuildAction(cx: ActionContext<Request>) -> Result<Outcome<Request, Response>> {
	let cmd = cx
		.caller
		.with_state::<AncestorQuery<&CargoBuildCmd>, _>(|entity, query| {
			query.get(entity).cloned()
		})
		.await?;
	let args = cmd.get_args();
	info!("building: {} {}", cmd.program, args.join(" "));
	ChildProcess::new(&cmd.program)
		.with_args(&args)
		.run_async()
		.await
		.map(|_| Pass(cx.input))
		.map_err(|err| bevyhow!("cargo build failed: {err}"))
}
