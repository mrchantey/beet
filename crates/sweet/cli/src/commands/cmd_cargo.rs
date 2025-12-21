use beet::prelude::*;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "test")]
pub struct CargoTest {
	#[command(flatten)]
	cargo_runner: CargoCmdExtra,
}
impl CargoTest {
	pub async fn run(mut self) -> Result {
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
	pub async fn run(mut self) -> Result {
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
	pub build_cmd: CargoBuildCmd,
	#[arg(short, long)]
	pub no_default_filters: bool,
	#[command(flatten)]
	pub filter: GlobFilter,
}


impl CargoCmdExtra {
	pub async fn run(mut self) -> Result {
		self.append_args();
		self.run_binary()?;
		Ok(())
	}

	fn append_args(&mut self) {
		if self.no_default_filters == false {
			self.filter
				.include("**/*.rs")
				.exclude("**/target/**")
				.exclude("**/.git/**")
				.exclude("*/codegen/*");
		}

		// temp until cli args
		let watch = true;
		if watch && self.build_cmd.lib {
			// watching, only works if before any trailing args
			self.build_cmd
				.trailing_args
				.insert(0, "--watch".to_string());
		}
	}
	// 	--include '**/*.rs' \
	// --exclude '{.git,target,html}/**' \
	// --exclude '*/codegen/*' \

	/// run the binary:
	/// ## Errors
	/// Errors if not in watch mode and the command fails
	fn run_binary(&self) -> Result {
		self.build_cmd.spawn()?;
		Ok(())
	}
}
