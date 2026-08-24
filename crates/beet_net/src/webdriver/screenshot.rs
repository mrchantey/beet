//! Screenshot capture via `browsingContext.captureScreenshot`.
//!
//! [`Page::screenshot`] captures the viewport, [`Element::screenshot`] clips
//! to one element, and [`ScreenshotOptions`] selects full-page or box clips,
//! mirroring the [`PdfOptions`] shape.

use super::Element;
use super::Page;
use super::*;
use base64::prelude::*;
use beet_core::prelude::*;
use serde_json::Value;
use serde_json::json;

/// Configuration options for screenshot capture. Always png, the only format
/// the BiDi spec requires of every browser.
#[derive(Debug, Default, Clone)]
pub struct ScreenshotOptions {
	/// The capture origin.
	pub origin: ScreenshotOrigin,
	/// An optional clip rectangle, in css px relative to [`Self::origin`].
	pub clip: Option<Rect>,
}

impl ScreenshotOptions {
	/// Capture the full document rather than the viewport.
	pub fn full_page() -> Self {
		Self {
			origin: ScreenshotOrigin::Document,
			..default()
		}
	}
}

/// The coordinate space and extent of a capture.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ScreenshotOrigin {
	/// Capture the visible viewport.
	#[default]
	Viewport,
	/// Capture the full document.
	Document,
}

impl Page {
	/// Capture the viewport as png bytes.
	pub async fn screenshot(&self) -> Result<Vec<u8>> {
		self.screenshot_with_options(&default()).await
	}

	/// Capture with custom options as png bytes.
	pub async fn screenshot_with_options(
		&self,
		options: &ScreenshotOptions,
	) -> Result<Vec<u8>> {
		let clip = options.clip.map(|clip| {
			json!({
				"type": "box",
				"x": clip.min.x,
				"y": clip.min.y,
				"width": clip.width(),
				"height": clip.height(),
			})
		});
		capture(&self.session, &self.context_id, options.origin, clip).await
	}
}

impl Element {
	/// Capture just this element as png bytes.
	pub async fn screenshot(&self) -> Result<Vec<u8>> {
		let clip = json!({"type": "element", "element": self.shared_ref()?});
		capture(
			self.session(),
			self.context_id(),
			// element clips are resolved against the full document, so a
			// node scrolled out of view still captures
			ScreenshotOrigin::Document,
			Some(clip),
		)
		.await
	}
}

/// Issue the capture and decode the base64 png payload.
async fn capture(
	session: &Session,
	context_id: &str,
	origin: ScreenshotOrigin,
	clip: Option<Value>,
) -> Result<Vec<u8>> {
	let mut params = json!({
		"context": context_id,
		"origin": match origin {
			ScreenshotOrigin::Viewport => "viewport",
			ScreenshotOrigin::Document => "document",
		},
		"format": { "type": "image/png" },
	});
	if let Some(clip) = clip {
		params.set_field("clip", clip)?;
	}
	session
		.command("browsingContext.captureScreenshot", params)
		.await?
		.pointer("/result/data")
		.and_then(|data| data.as_str())
		.ok_or_else(|| bevyhow!("missing screenshot data in response"))?
		.xmap(|data| BASE64_STANDARD.decode(data))
		.map_err(|err| bevyhow!("failed to decode base64 screenshot: {err}"))
}

#[cfg(test)]
mod test {
	use crate::webdriver::test_fixtures;
	use crate::webdriver::*;
	use beet_core::prelude::*;

	const PNG_MAGIC: &[u8] = &[0x89, b'P', b'N', b'G'];

	#[beet_core::test(timeout_ms = 30_000)]
	#[ignore = "smoketest"]
	async fn captures_page_and_element() {
		let url = test_fixtures::page_url(
			"screenshot",
			r#"<html><body style="background: rgb(200, 50, 50)">
			<h1 id="title">Shot</h1>
			</body></html>"#,
		);
		let page = test_fixtures::visit(&url).await;

		let viewport = page.screenshot().await.unwrap();
		viewport.starts_with(PNG_MAGIC).xpect_true();

		let full = page
			.screenshot_with_options(&ScreenshotOptions::full_page())
			.await
			.unwrap();
		full.starts_with(PNG_MAGIC).xpect_true();

		let element = page.find("#title").await.screenshot().await.unwrap();
		element.starts_with(PNG_MAGIC).xpect_true();
		// the heading crops far smaller than the whole viewport
		(element.len() < viewport.len()).xpect_true();

		page.kill().await.unwrap();
	}
}
