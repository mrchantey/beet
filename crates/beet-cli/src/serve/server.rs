use anyhow::Result;
use clap::Parser;




#[derive(Debug, Parser)]
#[command(name = "server")]
pub struct Server {}

impl Server {
	pub fn run(&self) -> Result<()> {
		println!("Server is running");

		Ok(())
	}
}
