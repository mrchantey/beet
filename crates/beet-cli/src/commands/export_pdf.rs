use beet::prelude::*;
use clap::Parser;

/// Build the project
#[derive(Debug, Clone, Parser)]
pub struct ExportPdf {
	/// Input url (positional)
	/// Disable margins
	#[clap(long = "no-margin")]
	pub no_margin: bool,
	pub input: String,
	/// Output file (-o, --output)
	#[clap(short = 'o', long = "output",
	default_value = "file.pdf",
	 value_parser = clap::value_parser!(std::path::PathBuf))]
	pub output: std::path::PathBuf,
	/// Page ranges to export, e.g. "1-5, 8, 11-13", or leave empty to export all
	#[clap(short = 'r', long = "ranges")]
	pub page_ranges: Vec<String>,
}

impl ExportPdf {
	#[allow(unused)]
	pub async fn run(self) -> Result {
		App::default()
			.run_io_task(async move {
				let mut opts = PdfOptions {
					page_ranges: self.page_ranges,
					..default()
				};
				if self.no_margin {
					opts.margin = PdfMargin::none();
				}


				let (proc, page) = Page::visit(&self.input).await?;
				let bytes = page.export_pdf_with_options(&opts).await?;
				fs_ext::write_async(self.output, bytes).await?;
				Ok(())
			})
			.await
	}
}
