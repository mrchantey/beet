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
	cargo_cmd: CargoCmd,
}

impl BuildBinaries {
	/// run the built binary without recompiling
	pub fn run_native(&self) -> Result<()> {
		Command::new(&self.cargo_cmd.exe_path())
			.status()?
			.exit_ok()?;
		Ok(())
	}

	pub fn recompile(&self, html_dir: &Path) -> Result<()> {
		let build_wasm_cmd = if self.wasm {
			let mut cmd = self.cargo_cmd.clone();
			cmd.target = Some("wasm32-unknown-unknown".to_string());
			Some(cmd)
		} else {
			None
		};
		if self.mpa {
			// TODO only recollect routes if routes change?
			self.collect_routes.build_and_write()?;
		}
		println!("🥁 building native");
		self.cargo_cmd.spawn()?;
		if let Some(wasm_cmd) = &build_wasm_cmd {
			println!("🥁 building wasm");
			wasm_cmd.spawn()?;
			self.wasm_bindgen(&wasm_cmd.exe_path(), html_dir)?;
		}
		Ok(())
	}

	/// execute `wasm-bindgen` with inferred locations. The wasm_exe_path
	/// should be the path to the output of `cargo build`
	fn wasm_bindgen(
		&self,
		wasm_exe_path: &Path,
		html_dir: &Path,
	) -> Result<()> {
		Command::new("wasm-bindgen")
			.arg("--out-dir")
			.arg(html_dir.join("wasm"))
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
