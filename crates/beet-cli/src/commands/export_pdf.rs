use super::pdf_ext;
use crate::prelude::*;
use beet::prelude::webdriver::*;
use beet::prelude::*;

/// Request params for the [`ExportPdf`] command, surfaced in `--help`.
#[derive(Reflect, Default)]
#[reflect(Default)]
struct ExportPdfParams {
	/// Page width in `--unit`s (default A4 width).
	width: f64,
	/// Page height in `--unit`s (default A4 height).
	height: f64,
	/// Unit for `--width`/`--height`: `mm` (default) or `px` (96 dpi).
	unit: Option<String>,
	/// Whether tall content stretches the page to fit (`stretch`, default) or is
	/// clipped to the page size (`clip`).
	overflow: Option<String>,
	/// Glob of route paths to include, ie `--include="blog/**"`.
	include: Option<String>,
	/// Glob of route paths to exclude, ie `--exclude="draft/**"`.
	exclude: Option<String>,
	/// Query string appended to every page request, ie
	/// `--search-params="color-scheme=dark"`.
	search_params: Option<String>,
	/// Output file, defaults to `<site>/site.pdf`.
	output: Option<String>,
	/// Disable page margins.
	no_margin: bool,
}

/// Exports every page of a no-code site to a single PDF, in route order, via a
/// headless browser (webdriver).
///
/// Builds and validates the site, boots its declared `HttpServer` on an ephemeral
/// port, then prints each static route and merges the results. `--width`/`--height`
/// set the page size (in `--unit`s: `mm` default, or `px`), `--include`/`--exclude`
/// glob-filter the routes, and `--search-params` rides every request (eg to drive
/// `color-scheme`). `--overflow=clip` keeps one page per route at exactly
/// `width`x`height`; the default `stretch` flows tall content across as many
/// `width`x`height` pages as it needs.
///
/// ```sh
/// beet export-pdf site --width=1920 --height=1080 --unit=px \
///   --search-params="color-scheme=light"
/// ```
#[action(route = "export-pdf/*site", handler_only)]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(ParamsPartial = ParamsPartial::new::<ExportPdfParams>())]
pub async fn ExportPdf(cx: ActionContext<Request>) -> Result<Response> {
	let parts = cx.input.request_parts();
	let params = parts.params().parse_reflect::<ExportPdfParams>()?;
	let SiteEntry { site_dir, entry } = resolve_site(&site_arg(parts)?)?;
	let root =
		build_site(&cx.caller, parts.params(), site_dir.clone(), entry).await?;

	// validate before exporting, like `export-static`: a broken no-code site fails
	// with a non-zero exit rather than shipping a broken PDF.
	let report = check_routes(&cx.world(), root).await?;
	report.log();
	if report.has_errors() {
		return Response::status_text(
			StatusCode::INTERNAL_SERVER_ERROR,
			format!(
				"export aborted: {} render error(s), see log\n",
				report.error_count()
			),
		)
		.xok();
	}

	// resolve the page size (cm, the `PdfPageSize` unit) and matching browser
	// viewport (px), defaulting to A4 when a dimension is unset.
	let unit = PdfUnit::parse(params.unit.as_deref())?;
	let overflow = PdfOverflow::parse(params.overflow.as_deref())?;
	let (page_size, viewport) =
		resolve_size(params.width, params.height, unit);
	let mut options = PdfOptions {
		page_size,
		..default()
	};
	if params.no_margin {
		options.margin = PdfMargin::none();
	}

	// the site's static pages, glob-filtered by `--include`/`--exclude`.
	let mut filter = GlobFilter::default();
	if let Some(include) = &params.include {
		filter = filter.with_include(include);
	}
	if let Some(exclude) = &params.exclude {
		filter = filter.with_exclude(exclude);
	}
	let paths = collect_static_paths(&cx.world(), root)
		.await?
		.into_iter()
		.filter(|path| filter.passes(path.as_str()))
		.collect::<Vec<_>>();
	if paths.is_empty() {
		bevybail!("no pages matched, nothing to export");
	}

	// boot the site's declared http server on an ephemeral port so the headless
	// browser fetches real, asset-resolved pages. the boot parks on the host's
	// `Running` keep-alive, so launch it fire-and-forget (driven by the app loop)
	// and wait for the bound port.
	cx.world()
		.run_async_local(move |world| async move {
			world
				.entity(root)
				.call::<Boot, Response>(Boot::from(Request::from_cli_str(
					"--server=http --port=0",
				)))
				.await?;
			Ok(())
		})
		.await;
	let port = wait_for_port().await?;

	// the query rides every request verbatim, eg `?color-scheme=dark`.
	let query = params
		.search_params
		.as_deref()
		.filter(|search| !search.is_empty())
		.map(|search| format!("?{search}"))
		.unwrap_or_default();
	let base = format!("http://127.0.0.1:{port}");
	let pdfs =
		export_pages(&paths, &base, &query, viewport, overflow, &options).await?;

	// stop the server (the teardown observer drops its listener) and write the
	// merged document.
	cx.world()
		.with(move |world: &mut World| {
			world.entity_mut(root).remove::<Running<Response>>();
		})
		.await;

	let output = match params.output {
		Some(output) => AbsPathBuf::new(output)?,
		None => site_dir.join("site.pdf"),
	};
	fs_ext::write_async(&output, pdf_ext::merge(pdfs)?).await?;
	Response::ok_text(format!("exported {} pages to {output}\n", paths.len()))
		.xok()
}

/// Drives one headless browser session over `paths`, returning a single-page PDF
/// per route. Reusing the session (navigating between routes) avoids a browser
/// process per page.
async fn export_pages(
	paths: &[SmolPath],
	base: &str,
	query: &str,
	viewport: (u32, u32),
	overflow: PdfOverflow,
	options: &PdfOptions,
) -> Result<Vec<Vec<u8>>> {
	let process = ClientProcess::new()?;
	let session = process.client().new_session().await?;
	let mut page = Page::from_session(session).await?;
	page.set_viewport(viewport.0, viewport.1).await?;

	let mut pdfs = Vec::with_capacity(paths.len());
	for path in paths {
		let url = format!("{base}{}{query}", path.with_leading_slash());
		page.navigate(&url).await?;
		let mut options = options.clone();
		// `Clip` keeps only the first printed page (content beyond width x height is
		// cut off); `Stretch` leaves the default, so tall content flows across as
		// many width x height pages as it needs.
		if overflow == PdfOverflow::Clip {
			options.page_ranges = vec!["1".to_string()];
		}
		pdfs.push(page.export_pdf_with_options(&options).await?);
	}
	page.kill().await?;
	process.kill()?;
	Ok(pdfs)
}

/// Polls the process-global server port until the booted http server binds,
/// erroring after ~5s rather than hanging.
async fn wait_for_port() -> Result<u16> {
	for _ in 0..200 {
		if let Ok(port) = HttpServer::current_port() {
			return Ok(port);
		}
		time_ext::sleep_millis(25).await;
	}
	bevybail!("http server did not bind within 5s")
}

/// Resolves the `(page_size_cm, viewport_px)` pair from the `--width`/`--height`
/// params (`0` = unset), defaulting each dimension to A4.
fn resolve_size(
	width: f64,
	height: f64,
	unit: PdfUnit,
) -> (PdfPageSize, (u32, u32)) {
	let a4 = PdfPageSize::a4();
	let (width_cm, viewport_w) = match width {
		width if width > 0.0 => (unit.to_cm(width), unit.to_px(width)),
		_ => (a4.width, PdfUnit::cm_to_px(a4.width)),
	};
	let (height_cm, viewport_h) = match height {
		height if height > 0.0 => (unit.to_cm(height), unit.to_px(height)),
		_ => (a4.height, PdfUnit::cm_to_px(a4.height)),
	};
	(
		PdfPageSize::custom(width_cm, height_cm),
		(viewport_w as u32, viewport_h as u32),
	)
}

/// Unit for `--width`/`--height`: millimetres (default) or CSS pixels at 96 dpi.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
enum PdfUnit {
	#[default]
	Mm,
	Px,
}

impl PdfUnit {
	/// CSS pixels per inch, the standard `px` reference.
	const DPI: f64 = 96.0;

	/// Parse the `--unit` param, defaulting to `Mm` when unset.
	fn parse(value: Option<&str>) -> Result<Self> {
		match value.map(str::trim) {
			None | Some("") | Some("mm") => Ok(Self::Mm),
			Some("px") => Ok(Self::Px),
			Some(other) => {
				bevybail!("unknown --unit '{other}', expected 'mm' or 'px'")
			}
		}
	}

	/// Convert a value in this unit to centimetres (the [`PdfPageSize`] unit).
	fn to_cm(self, value: f64) -> f64 {
		match self {
			Self::Mm => value / 10.0,
			Self::Px => Self::px_to_cm(value),
		}
	}

	/// Convert a value in this unit to CSS pixels (the viewport unit).
	fn to_px(self, value: f64) -> f64 {
		match self {
			Self::Mm => value / 25.4 * Self::DPI,
			Self::Px => value,
		}
	}

	/// CSS pixels to centimetres at 96 dpi.
	fn px_to_cm(px: f64) -> f64 { px / Self::DPI * 2.54 }

	/// Centimetres to CSS pixels at 96 dpi.
	fn cm_to_px(cm: f64) -> f64 { cm / 2.54 * Self::DPI }
}

/// How content taller than the page is handled.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
enum PdfOverflow {
	/// Let the document flow across as many width x height pages as it needs
	/// (the default), so nothing is lost.
	#[default]
	Stretch,
	/// Keep one width x height page per route, clipping any content beyond it.
	Clip,
}

impl PdfOverflow {
	/// Parse the `--overflow` param, defaulting to `Stretch` when unset.
	fn parse(value: Option<&str>) -> Result<Self> {
		match value.map(str::trim) {
			None | Some("") | Some("stretch") => Ok(Self::Stretch),
			Some("clip") => Ok(Self::Clip),
			Some(other) => bevybail!(
				"unknown --overflow '{other}', expected 'stretch' or 'clip'"
			),
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[beet::test]
	fn parses_units_and_overflow() {
		PdfUnit::parse(None).unwrap().xpect_eq(PdfUnit::Mm);
		PdfUnit::parse(Some("px")).unwrap().xpect_eq(PdfUnit::Px);
		PdfUnit::parse(Some("bad")).xpect_err();
		PdfOverflow::parse(None).unwrap().xpect_eq(PdfOverflow::Stretch);
		PdfOverflow::parse(Some("clip"))
			.unwrap()
			.xpect_eq(PdfOverflow::Clip);
		PdfOverflow::parse(Some("bad")).xpect_err();
	}

	#[beet::test]
	fn px_size_round_trips_to_viewport() {
		// 1920x1080 px → the viewport is exactly that, and the cm page size
		// converts back to the same px (the print width matches the measured width).
		let (page_size, viewport) = resolve_size(1920.0, 1080.0, PdfUnit::Px);
		viewport.xpect_eq((1920, 1080));
		PdfUnit::cm_to_px(page_size.width).xpect_close(1920.0);
		PdfUnit::cm_to_px(page_size.height).xpect_close(1080.0);
	}

	#[beet::test]
	fn unset_size_defaults_to_a4() {
		let (page_size, _) = resolve_size(0.0, 0.0, PdfUnit::Mm);
		page_size.width.xpect_close(21.0);
		page_size.height.xpect_close(29.7);
	}
}
