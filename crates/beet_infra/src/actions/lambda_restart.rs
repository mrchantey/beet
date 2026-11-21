//! `<LambdaRestart/>`: a function's warm execution environments retired.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// `<LambdaRestart/>`: after a content sync, retire the function's warm
/// execution environments, so the next request boots afresh from the repo
/// store.
///
/// The Lambda counterpart of `<LightsailRestart/>`, for the same reason: a
/// page's bytes are read from the store per request, but a boot resolves some
/// things ONCE: `<RoutesDir/>` discovery, the entry document itself, and the
/// derived files a declaration reads when it is declared (a
/// `<StandardSitePub>`'s index). A warm environment keeps the world it built
/// until Lambda retires it, so without this a sync's new page or index serves
/// only once the function happens to go cold.
///
/// Lambda has no restart call, so this points the function at the code it
/// already runs, the live version's artifact ([`AdoptCurrentDeploy`] names
/// it), and waits for the update to land: every invocation after it runs in a
/// fresh environment. The bucket, key and code are the ones the stack's tofu
/// state holds, so the next plan sees no drift.
///
/// Declared under the sync sequence after the [`RepoSync`] that publishes and
/// before the `<CloudflarePurgeCache/>`: purging while the old environments
/// still serve re-caches their responses.
#[action]
#[derive(Debug, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn LambdaRestart(
	/// The function's block label, the [`LambdaBlock`] default unless its
	/// stack declares another.
	#[field(default = LambdaBlock::default().label().clone())]
	label: SmolStr,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let (region, function, bucket, key) = cx
		.caller
		.with_state::<(StackQuery, RepoStoreQuery), _>(
			move |entity, (stacks, repos)| -> Result<_> {
				let stack = stacks.resolve(entity);
				let block = LambdaBlock::default().with_label(label);
				(
					stack.aws_region()?.clone(),
					block.ident(&stack).primary_identifier().to_string(),
					repos.get(entity)?.bucket_name()?,
					block.code_key(&stacks.deploy_id()),
				)
					.xok()
			},
		)
		.await??;
	aws_cli_ext::service("lambda", &region, [
		"update-function-code",
		"--function-name",
		&function,
		"--s3-bucket",
		&bucket,
		"--s3-key",
		key.as_str(),
	])
	.run_async()
	.await?;
	// the update is asynchronous: an invocation before it lands still meets
	// a warm environment
	aws_cli_ext::service("lambda", &region, [
		"wait",
		"function-updated",
		"--function-name",
		&function,
	])
	.run_async()
	.await?;
	info!("restarted {function}: the next request boots from the repo store");
	Pass(cx.input).xok()
}
