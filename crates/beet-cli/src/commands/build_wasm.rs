use beet::prelude::*;

/// The route bundle for the `build-wasm` command.
///
/// Configures a [`BuildWasm`] action from the CLI args (`--release`,
/// `--package`, `--out-dir`) and serves it at `build-wasm`. The action reads its
/// own [`BuildWasm`] state when dispatched.
pub fn build_wasm_route() -> impl Bundle {
	let args = CliArgs::parse_env();

	let mut cargo =
		CargoBuild::default().with_release(args.params.contains_key("release"));
	if let Some(package) = args.params.get("package") {
		cargo = cargo.with_package(package.clone());
	}

	(
		PathPartial::new("build-wasm"),
		BuildWasm::new(out_dir(&args)).with_cargo(cargo),
		ExchangeAction::new::<(), String, _, _>(),
	)
}

/// Resolves the wasm output directory from `--out-dir`, defaulting to `dist`
/// relative to the current directory.
fn out_dir(args: &CliArgs) -> AbsPathBuf {
	let raw = args
		.params
		.get("out-dir")
		.map(String::as_str)
		.unwrap_or("dist");
	AbsPathBuf::new(raw).unwrap_or_else(|_| AbsPathBuf::new_unchecked(raw))
}
