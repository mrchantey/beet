use crate::prelude::*;
use base64::prelude::*;
use beet_core::prelude::*;
use serde_json::json;


/// Configuration options for PDF export.
#[derive(Debug, Clone)]
pub struct PdfOptions {
	/// Whether to include background graphics.
	pub background: bool,
	/// Page margins in cm.
	pub margin: PdfMargin,
	/// Page orientation.
	pub orientation: PdfOrientation,
	/// Page size in cm.
	pub page_size: PdfPageSize,
	/// Scale factor for content (0.1 to 2.0).
	pub scale: f64,
	/// Whether to shrink content to fit page.
	pub shrink_to_fit: bool,
	/// Specific page ranges to include (e.g., ["1-3", "5"]).
	pub page_ranges: Vec<String>,
}


impl Default for PdfOptions {
	fn default() -> Self {
		Self {
			background: true,
			margin: default(),
			orientation: PdfOrientation::Portrait,
			page_size: PdfPageSize::a4(),
			scale: 1.0,
			page_ranges: Vec::new(),
			shrink_to_fit: true,
		}
	}
}


/// Page margins for PDF export (in cm).
#[derive(Debug, Clone)]
pub struct PdfMargin {
	pub top: f64,
	pub bottom: f64,
	pub left: f64,
	pub right: f64,
}

impl Default for PdfMargin {
	fn default() -> Self {
		Self {
			top: 1.0,
			bottom: 1.0,
			left: 1.0,
			right: 1.0,
		}
	}
}


impl PdfMargin {
	/// No margins (0 cm).
	pub fn none() -> Self {
		Self {
			top: 0.0,
			bottom: 0.0,
			left: 0.0,
			right: 0.0,
		}
	}
}


/// Page orientation for PDF export.
#[derive(Debug, Clone)]
pub enum PdfOrientation {
	Portrait,
	Landscape,
}

/// Page size for PDF export (in cm).
#[derive(Debug, Clone)]
pub struct PdfPageSize {
	pub width: f64,
	pub height: f64,
}

impl PdfPageSize {
	/// A4 page size (21.0 x 29.7 cm).
	pub fn a4() -> Self {
		Self {
			width: 21.0,
			height: 29.7,
		}
	}

	/// Letter page size (21.59 x 27.94 cm).
	pub fn letter() -> Self {
		Self {
			width: 21.59,
			height: 27.94,
		}
	}

	/// Legal page size (21.59 x 35.56 cm).
	pub fn legal() -> Self {
		Self {
			width: 21.59,
			height: 35.56,
		}
	}

	/// Custom page size.
	pub fn custom(width: f64, height: f64) -> Self { Self { width, height } }
}


impl Page {
	/// Export the current page as a PDF using default settings.
	/// Returns the PDF as raw bytes.
	pub async fn export_pdf(&self) -> Result<Vec<u8>> {
		self.export_pdf_with_options(&PdfOptions::default()).await
	}

	/// Export the current page as a PDF with custom options.
	/// Returns the PDF as raw bytes.
	pub async fn export_pdf_with_options(
		&self,
		options: &PdfOptions,
	) -> Result<Vec<u8>> {
		let response = self
			.session
			.command(
				"browsingContext.print",
				json!({
					"context": self.context_id,
					"background": options.background,
					"margin": {
						"top": options.margin.top,
						"bottom": options.margin.bottom,
						"left": options.margin.left,
						"right": options.margin.right
					},
					"orientation": match options.orientation {
						PdfOrientation::Portrait => "portrait",
						PdfOrientation::Landscape => "landscape",
					},
					"page": {
						"width": options.page_size.width,
						"height": options.page_size.height
					},
					"pageRanges": options.page_ranges,
					"scale": options.scale,
					"shrinkToFit": options.shrink_to_fit
				}),
			)
			.await?;

		let data_base64 = response
			.pointer("/result/data")
			.and_then(|v| v.as_str())
			.ok_or_else(|| bevyhow!("missing PDF data in response"))?;

		BASE64_STANDARD
			.decode(data_base64)
			.map_err(|e| bevyhow!("failed to decode base64 PDF data: {}", e))
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn export_pdf_generates_valid_pdf() {
		App::default()
			.run_io_task(async move {
				let (proc, page) =
					Page::visit("https://example.com").await.unwrap();

				let pdf_bytes = page.export_pdf().await.unwrap();

				// Basic validation that we got PDF bytes
				pdf_bytes.len().xpect_greater_than(0);

				// Check PDF header (PDF files start with "%PDF-")
				let header = std::str::from_utf8(&pdf_bytes[..4]).unwrap();
				header.xpect_eq("%PDF");

				page.kill().await.unwrap();
				proc.kill().await.unwrap();
			})
			.await;
	}

	#[sweet::test]
	async fn export_pdf_with_custom_options() {
		App::default()
			.run_io_task(async move {
				let (proc, page) =
					Page::visit("https://example.com").await.unwrap();

				let options = PdfOptions {
					background: false,
					margin: PdfMargin {
						top: 2.0,
						bottom: 2.0,
						left: 1.5,
						right: 1.5,
					},
					orientation: PdfOrientation::Landscape,
					page_size: PdfPageSize::letter(),
					scale: 0.8,
					shrink_to_fit: false,
					page_ranges: vec!["1".to_string()],
				};

				let pdf_bytes =
					page.export_pdf_with_options(&options).await.unwrap();

				// Basic validation that we got PDF bytes
				pdf_bytes.len().xpect_greater_than(0);

				// Check PDF header (PDF files start with "%PDF-")
				let header = std::str::from_utf8(&pdf_bytes[..4]).unwrap();
				header.xpect_eq("%PDF");

				page.kill().await.unwrap();
				proc.kill().await.unwrap();
			})
			.await;
	}
}
