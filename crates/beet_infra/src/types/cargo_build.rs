use crate::prelude::*;
use beet_core::prelude::*;
use std::path::PathBuf;

/// Information for executing a `cargo build` command
/// and resolving the executable path.
/// This type is transitory, used to construct a [`BuildArtifact`].
#[derive(Debug, Clone, Default, SetWith)]
pub struct CargoBuild {
	/// Build in release mode with optimizations.
	pub release: bool,
	/// Target wasm32-unknown-unknown.
	pub wasm: bool,
	/// Used as `-p my-crate`
	#[set_with(unwrap_option, into)]
	pub package: Option<SmolStr>,
	/// Name of the binary target
	#[set_with(unwrap_option, into)]
	pub binary: Option<SmolStr>,
	/// Name of the example target
	#[set_with(unwrap_option, into)]
	pub example: Option<SmolStr>,
	/// Root package used to resolve the binary name for a non-workspace main.rs
	#[set_with(unwrap_option, into)]
	pub root_crate_name: Option<SmolStr>,
	/// Additional arguments passed to cargo
	pub additional_args: Vec<SmolStr>,
}

impl CargoBuild {
	/// Resolve the binary name from the available fields.
	/// Panics if no name can be determined.
	pub fn binary_name(&self) -> SmolStr {
		if let Some(bin) = &self.binary {
			bin.clone()
		} else if let Some(example) = &self.example {
			example.clone()
		} else if let Some(pkg) = &self.package {
			pkg.clone()
		} else if let Some(name) = &self.root_crate_name {
			name.clone()
		} else {
			panic!("No binary, example, package, or root crate name provided")
		}
	}

	/// Resolve the expected executable path after a standard cargo build.
	pub fn exe_path(&self) -> PathBuf {
		let target_dir = env_ext::var("CARGO_TARGET_DIR")
			.unwrap_or_else(|_| "target".to_string());
		let mut path = PathBuf::from(target_dir);
		if self.wasm {
			path.push("wasm32-unknown-unknown");
		}
		if self.release {
			path.push("release");
		} else {
			path.push("debug");
		}
		if self.example.is_some() {
			path.push("examples");
		}
		path.push(self.binary_name().as_str());
		if self.wasm {
			path.set_extension("wasm");
		}
		path
	}

	/// Resolve the lambda build output directory.
	fn lambda_dir(&self) -> PathBuf {
		let target_dir = env_ext::var("CARGO_TARGET_DIR")
			.unwrap_or_else(|_| "target".to_string());
		let mut path = PathBuf::from(target_dir);
		path.push("lambda");
		path.push(self.binary_name().as_str());
		path
	}

	/// Resolve the expected executable path for a cargo-lambda build.
	pub fn lambda_exe_path(&self) -> PathBuf {
		self.lambda_dir().join("bootstrap")
	}

	/// Build the cargo command arguments.
	fn cargo_args(&self) -> Vec<SmolStr> {
		let mut args: Vec<SmolStr> = vec!["build".into()];
		if let Some(pkg) = &self.package {
			args.push("--package".into());
			args.push(pkg.clone());
		}
		if let Some(bin) = &self.binary {
			args.push("--bin".into());
			args.push(bin.clone());
		}
		if let Some(example) = &self.example {
			args.push("--example".into());
			args.push(example.clone());
		}
		if self.release {
			args.push("--release".into());
		}
		if self.wasm {
			args.push("--target".into());
			args.push("wasm32-unknown-unknown".into());
		}
		for arg in &self.additional_args {
			args.push(arg.clone());
		}
		args
	}

	/// Build the cargo-lambda command arguments.
	fn lambda_args(&self) -> Vec<SmolStr> {
		let mut args: Vec<SmolStr> = vec!["lambda".into(), "build".into()];
		if let Some(pkg) = &self.package {
			args.push("--package".into());
			args.push(pkg.clone());
		}
		if let Some(bin) = &self.binary {
			args.push("--bin".into());
			args.push(bin.clone());
		}
		if let Some(example) = &self.example {
			args.push("--example".into());
			args.push(example.clone());
		}
		if self.release {
			args.push("--release".into());
		}
		for arg in &self.additional_args {
			args.push(arg.clone());
		}
		args
	}

	/// Convert into a standard cargo [`BuildArtifact`].
	pub fn into_build_artifact(self) -> BuildArtifact {
		let artifact_path = self.exe_path();
		let args = self.cargo_args();
		BuildArtifact::new(
			ChildProcess::new("cargo").with_args(args),
			artifact_path,
		)
	}

	/// Convert into a lambda [`BuildArtifact`].
	/// Builds the lambda binary then zips it for S3 deployment,
	/// as AWS Lambda requires ZIP packages.
	pub fn into_lambda_build_artifact(self) -> BuildArtifact {
		let lambda_dir = self.lambda_dir();
		let zip_path = lambda_dir.join("bootstrap.zip");
		let args = self.lambda_args();
		// build the full command: cargo lambda build ... && zip the result
		let cargo_cmd = std::iter::once(SmolStr::from("cargo"))
			.chain(args)
			.collect::<Vec<SmolStr>>()
			.join(" ");
		let zip_cmd = format!(
			"cd {} && zip -j bootstrap.zip bootstrap",
			lambda_dir.display()
		);
		let full_cmd = format!("{cargo_cmd} && {zip_cmd}");
		BuildArtifact::new(
			ChildProcess::new("sh")
				.with_args([SmolStr::from("-c"), SmolStr::from(full_cmd)]),
			zip_path,
		)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn exe_path_example_release() {
		let build = CargoBuild::default()
			.with_release(true)
			.with_example("my-app");
		build.binary_name().as_str().xpect_eq("my-app");
		let path = build.exe_path();
		// exact path depends on CARGO_TARGET_DIR, just check it ends correctly
		path.ends_with("release/examples/my-app").xpect_true();
	}

	#[test]
	fn lambda_exe_path_example() {
		let build = CargoBuild::default().with_example("router");
		let path = build.lambda_exe_path();
		path.ends_with("lambda/router/bootstrap").xpect_true();
	}
}
