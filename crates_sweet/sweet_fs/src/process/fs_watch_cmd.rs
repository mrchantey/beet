use super::FsWatcher;
use crate::terminal;
use anyhow::Result;
use clap::Parser;

/// An [FsWatcher] that will run a command on change.
#[derive(Debug, Clone, Parser)]
pub struct FsWatchCmd {
	#[command(flatten)]
	watcher: FsWatcher,
	/// the command to run on change. This will run before any provided
	/// `on_change` callback.
	#[arg(long)]
	pub cmd: Option<String>,
}


impl FsWatchCmd {
	/// Run the command once, then watch, printing the
	/// mutated file each time.
	pub async fn run_and_watch(&self) -> Result<()> {
		terminal::clear().unwrap();
		println!("{:#?}", self);
		self.try_run_cmd().ok();
		self.watcher
			.watch_async(|e| {
				if let Some(mutated) = e.mutated_pretty() {
					terminal::clear().unwrap();
					println!("{}", mutated);
					self.try_run_cmd().ok();
				}
				Ok(())
			})
			.await?;
		Ok(())
	}

	fn try_run_cmd(&self) -> Result<()> {
		if let Some(cmd) = &self.cmd {
			let cmd_vec = cmd.split_whitespace().collect::<Vec<_>>();
			let status = std::process::Command::new(&cmd_vec[0])
				.args(&cmd_vec[1..])
				.status()?;

			if !status.success() {
				return Err(anyhow::anyhow!(
					"Command failed: {}\n{}",
					cmd,
					status
				));
			}
		}
		Ok(())
	}
}
