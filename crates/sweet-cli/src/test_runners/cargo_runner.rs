use anyhow::Result;
use clap::Parser;
use sweet::fs::terminal;
use sweet::prelude::CargoBuildCmd;
use sweet::prelude::FsWatcher;
use sweet::prelude::GlobFilter;

#[derive(Debug, Parser)]
#[command(name = "test")]
pub struct CargoTest {
	#[command(flatten)]
	cargo_runner: CargoCmdExtra,
}
impl CargoTest {
	pub async fn run(mut self) -> Result<()> {
		self.cargo_runner.build_cmd.cmd = "test".to_string();
		self.cargo_runner.run().await?;
		Ok(())
	}
}

#[derive(Debug, Parser)]
#[command(name = "run")]
pub struct CargoRun {
	#[command(flatten)]
	cargo_runner: CargoCmdExtra,
}
impl CargoRun {
	pub async fn run(mut self) -> Result<()> {
		self.cargo_runner.build_cmd.cmd = "run".to_string();
		self.cargo_runner.run().await?;
		Ok(())
	}
}

/// A cargo command with extra functionality like watch
#[derive(Debug, Parser)]
pub struct CargoCmdExtra {
	/// the file passed in by cargo test.
	///
	/// It will look something like $CARGO_TARGET_DIR/wasm32-unknown-unknown/debug/deps/hello_test-c3298911e67ad05b.wasm
	#[command(flatten)]
	build_cmd: CargoBuildCmd,
	#[arg(short, long)]
	watch: bool,
	#[arg(short, long)]
	no_default_filters: bool,
	#[command(flatten)]
	filter: GlobFilter,
}


impl CargoCmdExtra {
	pub async fn run(mut self) -> Result<()> {
		self.append_args();
		self.run_binary()?;
		if self.watch {
			self.watch().await?;
		}
		Ok(())
	}

	fn append_args(&mut self) {
		if self.no_default_filters == false {
			self.filter
				.include("**/*.rs")
				.exclude("{.git,target,html}/**")
				.exclude("*/codegen/*");
		}
		let is_upstream = self
			.build_cmd
			.package
			.as_ref()
			// these crates are upstream of sweet test so do not support the watch command
			.map(|p| ["sweet_utils", "sweet_fs"].contains(&p.as_str()))
			.unwrap_or(false);
		if self.watch && self.build_cmd.lib && !is_upstream {
			// watching
			self.build_cmd.push_cargo_args("--watch".to_string());
		}
	}
	// 	--include '**/*.rs' \
	// --exclude '{.git,target,html}/**' \
	// --exclude '*/codegen/*' \

	async fn watch(self) -> Result<()> {
		FsWatcher {
			filter: self.filter.clone(),
			..Default::default()
		}
		.watch_async(|ev| {
			if !ev.has_mutate() {
				return Ok(());
			}
			self.run_binary()?;
			Ok(())
		})
		.await?;
		Ok(())
	}

	fn run_binary(&self) -> Result<()> {
		if self.watch {
			terminal::clear()?;
		}
		self.build_cmd.spawn()?;
		Ok(())
	}
}
