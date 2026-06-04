//! Build artifact step for deploy sequences.
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use std::path::PathBuf;

/// A build step that runs a process and produces an artifact file.
/// Used as an ECS Component on deploy sequence entities alongside a block.
/// The [`TofuApplyAction`] collects these to build the artifact ledger.
#[derive(Debug, Clone, Get, SetWith, Component)]
#[require(BuildArtifactAction)]
pub struct BuildArtifact {
	/// The build command to execute
	process: ChildProcess,
	/// Label identifying this artifact, used as the key in [`ArtifactLedger`].
	/// Defaults to file stem, but must be overwritten as required, for instance
	/// [`LambdaBlock::label`].
	label: SmolStr,
	/// Path to the expected output artifact
	artifact_path: PathBuf,
}

impl BuildArtifact {
	/// Create a new build artifact from a process and expected output path.
	/// The label defaults to the file stem of the artifact path.
	pub fn new(
		process: ChildProcess,
		artifact_path: impl Into<PathBuf>,
	) -> Self {
		let artifact_path = artifact_path.into();
		let label: SmolStr = artifact_path
			.file_stem()
			.map(|stem| stem.to_string_lossy())
			.unwrap_or_else(|| "artifact".into())
			.into();
		Self {
			process,
			label,
			artifact_path,
		}
	}

	/// Compute a base64-encoded SHA256 hash of the artifact file,
	/// matching Terraform's `filebase64sha256` function.
	///
	/// ## Errors
	/// - Errors if the file cannot be read.
	/// - Errors if the deploy feature not enabled.
	pub fn compute_source_hash(&self) -> Result<String> {
		cfg_if! {
			if #[cfg(feature = "deploy")] {
				use base64::Engine;
				use sha2::Digest;
				let bytes = std::fs::read(&self.artifact_path)
					.map_err(|err| bevyhow!(
						"failed to read artifact {}: {err}",
						self.artifact_path.display()
					))?;
				let hash = sha2::Sha256::digest(&bytes);
				base64::engine::general_purpose::STANDARD.encode(hash).xok()
			} else {
				bevybail!("the `deploy` feature is required for artifact hash computation")
			}
		}
	}
}

/// Runs the build process from [`BuildArtifact`].
/// After building, the artifact file exists on disk for
/// [`TofuApplyAction`] to upload and hash.
#[action]
#[derive(Default, Component)]
pub async fn BuildArtifactAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	// read build artifact config
	let build = cx.caller.get_cloned::<BuildArtifact>().await?;

	// run the build process
	info!("building: {}", build.process());
	build
		.process()
		.clone()
		.run_async()
		.await
		.map_err(|err| bevyhow!("build failed: {err}"))?;

	let artifact_path = AbsPathBuf::new(build.artifact_path())?;
	info!("artifact built: {}", artifact_path.display());

	Pass(cx.input).xok()
}
