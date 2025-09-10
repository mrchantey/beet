use beet::prelude::*;
use clap::Parser;

/// Build the project
#[derive(Debug, Clone, Parser)]
pub struct ExportPdf {
	/// Input url (positional)
	pub input: String,

	/// Output file (-o, --output)
	#[clap(short = 'o', long = "output", value_parser = clap::value_parser!(std::path::PathBuf))]
	pub output: Option<std::path::PathBuf>,
}



impl ExportPdf {
	#[allow(unused)]
	pub async fn run(self) -> Result {
		todo!("chrome devtools protocol");
		// let devtools = ChromeDevTools::connect().await?;
		// let bytes = devtools.visit(&self.input).await?.export_pdf().await?;
		// let output = self.output.unwrap_or("output.pdf".into());
		// FsExt::write_async(output, bytes).await?;
		// Ok(())
	}
}
