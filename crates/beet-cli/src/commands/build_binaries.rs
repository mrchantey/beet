use crate::prelude::*;
use anyhow::Result;
use beet_router::prelude::CollectRoutes;
use clap::Parser;
use std::path::Path;
use std::process::Command;

/// Build both the server and wasm client binaries
/// for a beet app.

/// Serve a html application as either a spa or mpa
#[derive(Debug, Parser)]
pub struct BuildBinaries {
	/// enable default route collection
	#[arg(long)]
	mpa: bool,
	/// If the site contains reactivity, also build the client side wasm
	#[arg(long)]
	wasm: bool,
	/// if --mpa is passed, also regenerate routes before
	/// recompile
	#[command(flatten)]
	collect_routes: CollectRoutes,
	/// 🦀 the commands that will be used to build the html files 🦀
	#[command(flatten)]
	cargo_cmd: BuildCmd,
}

impl BuildBinaries {
	/// run the built binary without recompiling
	pub fn run_native(&self, watch_args: &WatchArgs) -> Result<()> {
		let mut cmd = Command::new(&self.cargo_cmd.exe_path());
		cmd.arg("--html-dir").arg(&watch_args.html_dir);

		if watch_args.as_static {
			cmd.arg("--static");
		}

		cmd.status()?.exit_ok()?;
		Ok(())
	}

	pub fn recompile(&self, watch_args: &WatchArgs) -> Result<()> {
		if self.mpa {
			// TODO only recollect routes if routes change?
			self.collect_routes.build_and_write()?;
		}
		println!("🥁 building native");
		self.cargo_cmd.spawn()?;

		if self.wasm {
			let mut cmd = self.cargo_cmd.clone();
			cmd.target = Some("wasm32-unknown-unknown".to_string());
			println!("🥁 building wasm");
			cmd.spawn()?;
			self.wasm_bindgen(&cmd.exe_path(), watch_args)?;
		}

		Ok(())
	}

	/// execute `wasm-bindgen` with inferred locations. The wasm_exe_path
	/// should be the path to the output of `cargo build`
	fn wasm_bindgen(
		&self,
		wasm_exe_path: &Path,
		watch_args: &WatchArgs,
	) -> Result<()> {
		Command::new("wasm-bindgen")
			.arg("--out-dir")
			.arg(watch_args.html_dir.join("wasm"))
			.arg("--out-name")
			.arg("bindgen")
			.arg("--target")
			.arg("web")
			.arg("--no-typescript")
			.arg(wasm_exe_path)
			.status()?
			.exit_ok()?;
		Ok(())
	}
}
