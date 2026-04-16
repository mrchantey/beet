//! Upload artifact step for deploy sequences.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Upload artifact step for deploy exchange sequences.
/// Reads the built binary from [`CargoBuildCmd::exe_path`],
/// uploads it to the [`ArtifactsBucket`], and sets it as current.
#[cfg(all(feature = "aws", feature = "bindings_aws_common"))]
#[action]
#[derive(Default, Component)]
pub async fn UploadArtifactAction(cx: ActionContext<Request>) -> Result<Outcome<Request, Response>> {
	// read build output path from CargoBuildCmd on ancestor
	let exe_path = cx
		.caller
		.with_state::<AncestorQuery<&CargoBuildCmd>, _>(|entity, query| {
			query.get(entity).map(|cmd| cmd.exe_path(None))
		})
		.await?;
	let exe_path = AbsPathBuf::new(exe_path)?;

	// read the binary
	let bytes = fs_ext::read_async(&exe_path).await?;
	info!("uploading artifact: {} ({} bytes)", exe_path.display(), bytes.len());

	// get the artifacts client
	let client = cx
		.caller
		.with_state::<StackQuery, _>(|entity, query| {
			query.artifacts_client(entity)
		})
		.await?;

	// upload and set as current
	let artifact_name = exe_path
		.file_name()
		.map(|name| name.to_string_lossy().to_string())
		.unwrap_or_else(|| "artifact".to_string());
	let version = client.upload(&artifact_name, bytes).await?;
	client.set_current(&version).await?;
	info!("artifact version: {version}");

	Pass(cx.input).xok()
}
