use beet::prelude::*;
use beet::prelude::webdriver::*;
use std::path::PathBuf;

/// Exports a URL to a PDF via a headless browser (webdriver).
///
/// `--input` is the URL, `--output` the file (default `file.pdf`),
/// `--no-margin` disables margins, and `--page-ranges` limits the pages, ie
/// `--page-ranges=1-5,8`.
#[action]
#[derive(Component)]
pub async fn ExportPdf(parts: RequestParts) -> Result<String> {
	let input = parts
		.get_param("input")
		.ok_or_else(|| bevyhow!("export-pdf requires --input"))?
		.to_string();
	let output = parts
		.get_param("output")
		.map(PathBuf::from)
		.unwrap_or_else(|| "file.pdf".into());

	let mut options = PdfOptions::default();
	if parts.has_param("no-margin") {
		options.margin = PdfMargin::none();
	}
	if let Some(ranges) = parts.get_param("page-ranges") {
		options.page_ranges =
			ranges.split(',').map(|range| range.trim().to_string()).collect();
	}

	let (_process, page) = Page::visit(&input).await?;
	let bytes = page.export_pdf_with_options(&options).await?;
	fs_ext::write_async(&output, bytes).await?;
	Ok(format!("wrote pdf to {}", output.display()))
}
