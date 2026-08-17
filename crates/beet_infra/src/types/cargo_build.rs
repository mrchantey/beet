#[allow(unused)]
use crate::prelude::*;
use beet_core::prelude::*;
use std::path::PathBuf;

/// Cross-compilation target for cargo builds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum BuildTarget {
	/// Native host target, no cross-compilation.
	#[default]
	Native,
	/// WebAssembly via wasm32-unknown-unknown.
	Wasm,
	/// Linux x86_64 glibc via cargo zigbuild.
	/// Uses zig as a linker for cross-compilation to x86_64-unknown-linux-gnu.
	Zigbuild,
}

impl BuildTarget {
	/// The rustc target triple, if cross-compiling.
	pub fn target_triple(&self) -> Option<&'static str> {
		match self {
			BuildTarget::Native => None,
			BuildTarget::Wasm => Some("wasm32-unknown-unknown"),
			BuildTarget::Zigbuild => Some("x86_64-unknown-linux-gnu"),
		}
	}

	/// Whether this target uses cargo-zigbuild instead of cargo build.
	pub fn uses_zigbuild(&self) -> bool {
		matches!(self, BuildTarget::Zigbuild)
	}
}

/// Information for executing a cargo build command
/// and resolving the executable path.
/// This type is transitory, used to construct a [BuildArtifact].
#[derive(Debug, Clone, Default, SetWith)]
pub struct CargoBuild {
	/// Build in release mode with optimizations.
	pub release: bool,
	/// Cross-compilation target.
	pub target: BuildTarget,
	/// Used as -p my-crate
	#[set_with(unwrap_option, into)]
	pub package: Option<SmolStr>,
	/// Name of the binary target.
	#[set_with(unwrap_option, into)]
	pub binary: Option<SmolStr>,
	/// Name of the example target.
	#[set_with(unwrap_option, into)]
	pub example: Option<SmolStr>,
	/// Root package used to resolve the binary name for a non-workspace main.rs.
	#[set_with(unwrap_option, into)]
	pub root_crate_name: Option<SmolStr>,
	/// Do not activate the `default` feature.
	pub no_default_features: bool,
	/// Activate all features, ie `--all-features`.
	pub all_features: bool,
	/// Features to activate, ie `--features client`.
	pub features: Vec<SmolStr>,
	/// Additional arguments passed to cargo.
	pub additional_args: Vec<SmolStr>,
	/// The *deployed* binary's [`BootstrapConfig`]. The lambda runtime offers no
	/// argv, so the zip converts at the container boundary: it ships a one-line
	/// `bootstrap` script exec'ing the binary with the config's argv channel (see
	/// [`into_lambda_build_artifact`](Self::into_lambda_build_artifact)). Unused by
	/// non-lambda artifacts, whose launch path owns its args.
	pub bootstrap: BootstrapConfig,
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
			.unwrap_or_else(|_| "target".into());
		let mut path = PathBuf::from(target_dir.as_str());
		// cross-compilation targets put output in a subdirectory
		if let Some(triple) = self.target.target_triple() {
			path.push(triple);
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
		if self.target == BuildTarget::Wasm {
			path.set_extension("wasm");
		}
		path
	}

	/// Resolve the lambda build output directory.
	fn lambda_dir(&self) -> PathBuf {
		let target_dir = env_ext::var("CARGO_TARGET_DIR")
			.unwrap_or_else(|_| "target".into());
		let mut path = PathBuf::from(target_dir.as_str());
		path.push("lambda");
		path.push(self.binary_name().as_str());
		path
	}

	/// Resolve the expected executable path for a cargo-lambda build.
	pub fn lambda_exe_path(&self) -> PathBuf {
		self.lambda_dir().join("bootstrap")
	}
	/// Build the cargo command arguments, beginning with the `build` subcommand.
	pub fn cargo_args(&self) -> Vec<SmolStr> {
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
		if let Some(triple) = self.target.target_triple() {
			args.push("--target".into());
			args.push(triple.into());
		}
		if self.no_default_features {
			args.push("--no-default-features".into());
		}
		if self.all_features {
			args.push("--all-features".into());
		} else if !self.features.is_empty() {
			args.push("--features".into());
			args.push(self.features.join(",").into());
		}
		for arg in &self.additional_args {
			args.push(arg.clone());
		}
		args
	}

	/// Build the cargo-lambda command arguments.
	#[cfg(feature = "deploy")]
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
	/// Convert into a standard cargo build artifact.
	/// For zigbuild targets, uses cargo-zigbuild instead of cargo build.
	#[cfg(feature = "deploy")]
	pub fn into_build_artifact(self) -> BuildArtifact {
		let artifact_path = self.exe_path();
		let args = self.cargo_args();
		// zigbuild uses cargo-zigbuild instead of cargo build
		let cmd = if self.target.uses_zigbuild() {
			"cargo-zigbuild"
		} else {
			"cargo"
		};
		BuildArtifact::new(
			ChildProcess::new(cmd).with_args(args),
			artifact_path,
		)
	}
	/// Convert into a lambda build artifact.
	/// Builds the lambda binary then zips it for S3 deployment,
	/// as AWS Lambda requires ZIP packages.
	///
	/// When target is [`BuildTarget::Zigbuild`], uses cargo-zigbuild to
	/// cross-compile, then packages the binary as `bootstrap.zip`.
	/// Otherwise falls back to cargo-lambda which handles cross-compilation
	/// and packaging itself.
	///
	/// With a non-empty [`bootstrap`](Self::bootstrap) the zip ships the binary as
	/// `beet` plus a one-line `bootstrap` script exec'ing it with the config's
	/// argv: the lambda runtime offers no argv, so the script is the env-to-args
	/// boundary.
	#[cfg(feature = "deploy")]
	pub fn into_lambda_build_artifact(mut self) -> Result<BuildArtifact> {
		if self.target == BuildTarget::Zigbuild {
			// zigbuild: cross-compile, then zip as bootstrap
			self.release = true;
			let exe_path = self.exe_path();
			let lambda_dir = self.lambda_dir();
			let zip_path = lambda_dir.join("bootstrap.zip");
			let args = self.cargo_args();
			let build_cmd = std::iter::once(SmolStr::from("cargo-zigbuild"))
				.chain(args)
				.collect::<Vec<SmolStr>>()
				.join(" ");
			// create lambda dir, copy binary as bootstrap, zip it
			let pack_cmd = format!(
				"mkdir -p {dir} && cp {exe} {dir}/bootstrap && {zip}",
				dir = lambda_dir.display(),
				exe = exe_path.display(),
				zip = self.lambda_zip_cmd(&lambda_dir)?,
			);
			let full_cmd = format!("{build_cmd} && {pack_cmd}");
			BuildArtifact::new(
				ChildProcess::new("sh")
					.with_args([SmolStr::from("-c"), SmolStr::from(full_cmd)]),
				zip_path,
			)
			.xok()
		} else {
			// cargo-lambda: build and zip in one step
			let lambda_dir = self.lambda_dir();
			let zip_path = lambda_dir.join("bootstrap.zip");
			let args = self.lambda_args();
			let cargo_cmd = std::iter::once(SmolStr::from("cargo"))
				.chain(args)
				.collect::<Vec<SmolStr>>()
				.join(" ");
			let full_cmd =
				format!("{cargo_cmd} && {}", self.lambda_zip_cmd(&lambda_dir)?);
			BuildArtifact::new(
				ChildProcess::new("sh")
					.with_args([SmolStr::from("-c"), SmolStr::from(full_cmd)]),
				zip_path,
			)
			.xok()
		}
	}

	/// The zip packaging tail, run with the built binary at `{dir}/bootstrap`.
	/// With a non-empty [`bootstrap`](Self::bootstrap) argv channel the binary
	/// shifts to `beet` and a one-line `bootstrap` script exec's it with the args
	/// (the lambda runtime offers no argv); without them the binary itself stays
	/// `bootstrap`.
	#[cfg(feature = "deploy")]
	fn lambda_zip_cmd(&self, dir: &std::path::Path) -> Result<String> {
		// `to_argv` validates every token against a shell-safe charset, so an arg
		// that would corrupt the exec line is a render error rather than a
		// silently broken deploy.
		let args = self
			.bootstrap
			.clone()
			.split_channels()
			.0
			.to_argv()?
			.iter()
			.map(|arg| format!(" {arg}"))
			.collect::<String>();
		if args.is_empty() {
			return format!("cd {} && zip -j bootstrap.zip bootstrap", dir.display())
				.xok();
		}
		format!(
			"cd {dir} && mv bootstrap beet && printf '#!/bin/sh\\nexec /var/task/beet{args}\\n' > bootstrap && chmod +x bootstrap && zip -j bootstrap.zip bootstrap beet",
			dir = dir.display(),
		)
		.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn exe_path_example_release() {
		let build = CargoBuild::default()
			.with_release(true)
			.with_example("my-app");
		build.binary_name().as_str().xpect_eq("my-app");
		let path = build.exe_path();
		// exact path depends on CARGO_TARGET_DIR, just check it ends correctly
		path.ends_with("release/examples/my-app").xpect_true();
	}

	#[beet_core::test]
	fn exe_path_zigbuild_example_release() {
		let build = CargoBuild::default()
			.with_release(true)
			.with_target(BuildTarget::Zigbuild)
			.with_example("my-app");
		let path = build.exe_path();
		path.ends_with("x86_64-unknown-linux-gnu/release/examples/my-app")
			.xpect_true();
	}

	/// The lambda `bootstrap` script exec's the binary with the config's argv
	/// channel; an empty config leaves the built binary as `bootstrap` itself.
	#[cfg(feature = "deploy")]
	#[beet_core::test]
	fn lambda_zip_cmd_renders_bootstrap_argv() {
		let dir = std::path::Path::new("/tmp/lam");
		CargoBuild::default()
			.lambda_zip_cmd(dir)
			.unwrap()
			.xpect_eq("cd /tmp/lam && zip -j bootstrap.zip bootstrap");
		CargoBuild::default()
			.with_bootstrap(BootstrapConfig {
				store: Some(StoreUri::parse("s3://beet--dev--app").unwrap()),
				server: Some(ServerFilter::new("http")),
				..default()
			})
			.lambda_zip_cmd(dir)
			.unwrap()
			.xpect_contains(
				"exec /var/task/beet --store=s3://beet--dev--app --server=http",
			);
	}

	/// A token that cannot be rendered into the exec line is a loud error rather
	/// than a corrupted `bootstrap` script: the old printf caveat, enforced.
	#[cfg(feature = "deploy")]
	#[beet_core::test]
	fn lambda_zip_cmd_rejects_unencodable_args() {
		CargoBuild::default()
			.with_bootstrap(BootstrapConfig {
				path: Some("/my page".into()),
				..default()
			})
			.lambda_zip_cmd(std::path::Path::new("/tmp/lam"))
			.unwrap_err()
			.to_string()
			.xpect_contains("cannot be rendered");
	}

	#[beet_core::test]
	fn lambda_exe_path_example() {
		let build = CargoBuild::default().with_example("router");
		let path = build.lambda_exe_path();
		path.ends_with("lambda/router/bootstrap").xpect_true();
	}

	#[beet_core::test]
	fn cargo_args_wasm_features() {
		CargoBuild::default()
			.with_target(BuildTarget::Wasm)
			.with_package("beet-cli")
			.with_binary("beet")
			.with_no_default_features(true)
			.with_features(vec!["web".into()])
			.cargo_args()
			.join(" ")
			.xpect_eq(
				"build --package beet-cli --bin beet --target wasm32-unknown-unknown \
				 --no-default-features --features web",
			);
	}

	// `--all-features` wins over an explicit `--features` list.
	#[beet_core::test]
	fn cargo_args_all_features() {
		CargoBuild::default()
			.with_all_features(true)
			.with_features(vec!["web".into()])
			.cargo_args()
			.join(" ")
			.xpect_eq("build --all-features");
	}
}
