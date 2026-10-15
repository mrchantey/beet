//! Building a wasm runtime as a deploy step.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// `<WasmRuntime package="beet-cli" bin="beet" features="web_ui" out="target/wasm/beet-ui.wasm"/>`:
/// a deploy step that builds a release wasm runtime, so the step after it
/// publishes the runtime this deploy built.
///
/// A runtime is never borrowed as a stored file: whatever an earlier build
/// (or a bucket pull) left at `out` is overwritten here, immediately before
/// the publish reads it. Declare it directly before the step that reads `out`,
/// ie a `<RepoStage>` whose source links to it, or an R2 sync of that source.
///
/// The build is a release [`BuildWasm::build`], the same one the
/// `build-wasm` command and the `just build-wasm-*` recipes run.
#[action]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn WasmRuntime(
	/// The cargo package to build (`--package`).
	#[field(required)]
	package: SmolStr,
	/// The binary target to build (`--bin`).
	#[field(required)]
	bin: SmolStr,
	/// The comma-separated cargo features the runtime is built with (`--features`).
	/// The build disables the package's default features, so only these are
	/// enabled.
	#[field]
	features: SmolStr,
	/// The workspace-relative `.wasm` path; its `.js` glue is written beside it.
	#[field(required)]
	out: WsPath,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	BuildWasm::default()
		.with_release(true)
		.with_package(package.to_string())
		.with_bin(bin.to_string())
		.with_features(features.to_string())
		.with_out(out.into_abs().to_string())
		.build()
		.await?;
	Pass(cx.input).xok()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_router::prelude::*;

	/// The step runs where it is declared: a deploy sequence calls it, so an
	/// unset field fails the verb by name. An action the sequence could not
	/// call would be skipped instead, and the publish after it would ship
	/// whatever an earlier build left at `out`.
	#[beet_core::test]
	async fn runs_as_a_deploy_step() {
		(AsyncPlugin, InfraPlugin)
			.into_world()
			.spawn((ExchangeSequence, children![WasmRuntime::default()]))
			.call::<Request, Outcome<Request, Response>>(Request::get(""))
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("missing required field `package`");
	}
}
