use beet::prelude::*;

/// Build the project
#[derive(Debug, Clone)]
pub struct ExportPdf {
	/// Input url (positional)
	/// Disable margins
	pub no_margin: bool,
	pub input: String,
	/// Output file (-o, --output)
	pub output: std::path::PathBuf,
	/// Page ranges to export, e.g. "1-5, 8, 11-13", or leave empty to export all
	pub page_ranges: Vec<String>,
}

impl Default for ExportPdf {
	fn default() -> Self {
		Self {
			no_margin: false,
			input: String::new(),
			output: "file.pdf".into(),
			page_ranges: Vec::new(),
		}
	}
}

impl ExportPdf {
	#[allow(unused)]
	pub async fn run(self) -> Result {
		App::default()
			.run_io_task_local(async move {
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
