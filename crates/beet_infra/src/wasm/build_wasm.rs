use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Compiles a package to wasm and prepares it for the browser.
///
/// On call, runs `cargo build --target wasm32-unknown-unknown
/// --no-default-features --features client`, then `wasm-bindgen` into
/// `<out_dir>/<wasm_dir>`, and in release mode `wasm-opt -Oz` over the result,
/// returning the artifact size.
#[derive(Debug, Clone, SetWith)]
pub struct BuildWasm {
	/// The cargo build configuration. The wasm target and `client` feature are
	/// applied when the build runs.
	pub cargo: CargoBuild,
	/// Root directory the `wasm-bindgen` output is written under.
	pub out_dir: AbsPathBuf,
	/// Directory, relative to [`Self::out_dir`], the wasm artifacts are written to.
	pub wasm_dir: RelPath,
	/// The `--out-name` passed to `wasm-bindgen`, ie `main` → `main_bg.wasm`.
	pub wasm_name: String,
}

/// Request params for the [`BuildWasm`] command, surfaced in `--help`.
#[derive(Reflect)]
struct BuildWasmParams {
	/// Build in release mode and optimize the artifact with `wasm-opt -Oz`.
	release: bool,
	/// The cargo package to build, ie `--package my-app`.
	package: Option<String>,
	/// Directory the `wasm-bindgen` output is written under, defaults to `dist`.
	out_dir: Option<String>,
}

impl BuildWasm {
	/// Creates a build writing to `out_dir`, with the default `wasm`/`main`
	/// artifact location.
	pub fn new(out_dir: AbsPathBuf) -> Self {
		Self {
			cargo: CargoBuild::default(),
			out_dir,
			wasm_dir: RelPath::new("wasm"),
			wasm_name: "main".into(),
		}
	}

	/// Builds a config from CLI/request params: `--release`, `--package`, and
	/// `--out-dir` (defaulting to `dist`).
	pub fn from_request_parts(parts: &RequestParts) -> Self {
		let mut cargo =
			CargoBuild::default().with_release(parts.has_param("release"));
		if let Some(package) = parts.get_param("package") {
			cargo = cargo.with_package(package);
		}
		let raw = parts.get_param("out-dir").unwrap_or("dist");
		let out_dir = AbsPathBuf::new(raw)
			.unwrap_or_else(|_| AbsPathBuf::new_unchecked(raw));
		Self::new(out_dir).with_cargo(cargo)
	}

	/// Runs the cargo → wasm-bindgen → wasm-opt pipeline, returning a size report.
	async fn run(&self) -> Result<String> {
		let cargo = self
			.cargo
			.clone()
			.with_target(BuildTarget::Wasm)
			.with_no_default_features(true)
			.with_features(vec!["client".into()]);

		// 1. cargo build
		ChildProcess::new("cargo")
			.with_args(cargo.cargo_args())
			.run_async()
			.await?;

		// 2. wasm-bindgen
		let wasm_out = self.out_dir.join(self.wasm_dir.to_string());
		ChildProcess::new("wasm-bindgen")
			.with_args([
				"--out-dir".to_string(),
				wasm_out.to_string_lossy().to_string(),
				"--out-name".to_string(),
				self.wasm_name.clone(),
				"--target".to_string(),
				"web".to_string(),
				"--no-typescript".to_string(),
				cargo.exe_path().to_string_lossy().to_string(),
			])
			.run_async()
			.await?;

		// 3. wasm-opt (release only)
		let wasm_file = wasm_out.join(format!("{}_bg.wasm", self.wasm_name));
		if cargo.release {
			ChildProcess::new("wasm-opt")
				.with_args([
					"-Oz".to_string(),
					"--output".to_string(),
					wasm_file.to_string(),
					wasm_file.to_string(),
				])
				.run_async()
				.await?;
		}

		let size_kb = std::fs::metadata(&wasm_file)
			.map(|meta| meta.len() as usize / 1024)
			.unwrap_or(0);
		let report = format!("🌱 wasm size: {size_kb} KB");
		info!("{report}");
		Ok(report)
	}
}

/// Builds a [`BuildWasm`] config from the request params and runs the build,
/// returning a size report.
#[action]
#[derive(Component)]
#[require(ParamsPartial = ParamsPartial::new::<BuildWasmParams>())]
pub async fn BuildWasmAction(parts: RequestParts) -> Result<String> {
	BuildWasm::from_request_parts(&parts).run().await
}
