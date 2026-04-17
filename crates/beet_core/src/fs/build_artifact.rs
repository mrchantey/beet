use crate::prelude::*;
use std::path::PathBuf;

/// A build step that runs a process and produces an artifact file.
/// Used as an ECS Component on deploy sequence entities.
#[derive(Debug, Clone, Get, SetWith, Component)]
pub struct BuildArtifact {
	/// The build command to execute
	process: ChildProcess,
	/// Path to the expected output artifact
	artifact_path: PathBuf,
}

impl BuildArtifact {
	/// Create a new build artifact from a process and expected output path.
	pub fn new(process: ChildProcess, artifact_path: impl Into<PathBuf>) -> Self {
		Self {
			process,
			artifact_path: artifact_path.into(),
		}
	}
}
