use beet::prelude::webdriver::*;
use beet::prelude::*;

/// Request params for the [`CaptureScreenshot`] command, surfaced in `--help`.
#[derive(Reflect, Default)]
#[reflect(Default)]
struct ScreenshotParams {
	/// Viewport width in px (default 1280).
	width: u32,
	/// Viewport height in px (default 800).
	height: u32,
	/// Capture the full document instead of the viewport.
	full_page: bool,
	/// Clip to the first element matching this css selector (auto-waits for
	/// it to appear).
	selector: Option<String>,
	/// Output path (default `screenshot.png`).
	output: Option<String>,
}

/// Captures a png of a url via a headless browser (webdriver): the viewport
/// by default, `--full-page` for the whole document, or `--selector` for one
/// element. Color schemes and other page state ride the url itself, eg
/// `?color-scheme=dark`.
///
/// ```sh
/// beet screenshot http://localhost:8337/docs --output=docs.png
/// beet screenshot 'http://localhost:8337/docs/design/counter?color-scheme=dark' \
///   --selector='#sidebar' --output=sidebar.png
/// ```
#[action(route = "screenshot/*url", handler_only)]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(ParamsPartial = ParamsPartial::new::<ScreenshotParams>())]
// `CaptureScreenshot` not `Screenshot`: bevy's render stack registers its own
// `Screenshot` component under the render features, and two short type paths
// cannot share a tag
pub async fn CaptureScreenshot(cx: ActionContext<Request>) -> Result<Response> {
	let parts = cx.input.request_parts();
	let params = parts.params().parse_reflect::<ScreenshotParams>()?;
	// like `run-wasm`, the greedy capture keeps the url as intact trailing
	// segments; rejoin and drop the command segment
	let mut cli = parts.to_cli_args();
	let mut segments = core::mem::take(&mut cli.path);
	if segments.len() < 2 {
		bevybail!("usage: beet screenshot <url> [--output=path.png]");
	}
	segments.remove(0);
	let url = segments.join("/");

	let width = match params.width {
		0 => 1280,
		width => width,
	};
	let height = match params.height {
		0 => 800,
		height => height,
	};
	let output = params
		.output
		.clone()
		.unwrap_or_else(|| "screenshot.png".to_string());

	// owned args + a helper fn: inline borrows across the awaits here trip
	// the action macro's `Send` bound ("implementation of `Send` is not
	// general enough"), the same shape `export_pdf` uses
	let png =
		capture(url, width, height, params.full_page, params.selector.clone())
			.await?;
	fs_ext::write(&output, &png)?;
	Response::ok_text(format!("wrote {output} ({} bytes)\n", png.len())).xok()
}

/// Drive one headless browser session and capture the requested png.
async fn capture(
	url: String,
	width: u32,
	height: u32,
	full_page: bool,
	selector: Option<String>,
) -> Result<Vec<u8>> {
	let (process, page) = Page::visit(&url).await?;
	page.set_viewport(width, height).await?;
	let png = match selector {
		Some(selector) => {
			let element = page.try_find(&selector).await?;
			element.screenshot().await?
		}
		None if full_page => {
			let options = ScreenshotOptions::full_page();
			page.screenshot_with_options(&options).await?
		}
		None => page.screenshot().await?,
	};
	page.kill().await?;
	process.kill()?;
	Ok(png)
}
