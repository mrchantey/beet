use beet_core::prelude::*;
use std::path::PathBuf;

/// A build step that runs a process and produces an artifact file.
/// Used as an ECS Component on deploy sequence entities.
#[derive(Debug, Clone, Get, SetWith, Component)]
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
}
