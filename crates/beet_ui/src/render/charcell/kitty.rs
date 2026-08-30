//! Raster images in the terminal via the kitty graphics protocol.
//!
//! Supported terminals (kitty, ghostty, WezTerm) draw real images over the
//! cell grid using APC escapes. `attach_kitty_images` fetches each `<img>`'s
//! `src` over HTTP — an absolute `http(s)://` directly, a site-rooted
//! `/assets/…` looped back to our own canonical server (which maps it to its
//! blob store), exactly as a browser resolves it against the document origin, so
//! there is no filesystem dependency on the render host (Lambda/Fargate). PNG
//! bytes transmit directly, an `<img src=*.svg>` is rasterised to PNG (resvg),
//! any other raster format decodes and re-encodes to PNG, then a [`KittyImage`]
//! and the `graphics` element state attach so the terminal-gated user-agent rule
//! gives it a block box. The measure and layout phases size that box from the
//! pixel dimensions, contained within its [`CellBounds`] so no raster wants a
//! box its scroll port could never show; paint reserves its cells; and
//! `place_kitty_images` transmits the bytes once and (re)places the picture
//! whenever its on-screen rect changes — scroll, reflow, or resize — cropping
//! to the visible part of a box the port only partly shows.
//!
//! On any failure (no canonical server, a refused/non-2xx fetch, a decode error)
//! the element shows both its `[image]: alt` marker and the styled error message
//! ([`render_image_errors`]); unsupported terminals keep just the marker.
//!
//! Protocol reference: <https://sw.kovidgoyal.net/kitty/graphics-protocol/>

#[cfg(feature = "tui")]
use super::*;
use crate::prelude::*;
use beet_core::prelude::*;
#[cfg(feature = "tui")]
use bevy::math::IRect;
#[cfg(feature = "tui")]
use bevy::math::URect;
use bevy::math::UVec2;
#[cfg(feature = "tui")]
use std::io::Write;

/// A raster attached to an `<img>` element: its kitty image id, base64-encoded
/// PNG payload, and pixel dimensions.
///
/// Data-only and platform-neutral (measure/paint read it on every target);
/// the systems that attach and emit it are `tui`-gated.
#[derive(Debug, Clone, Component)]
pub struct KittyImage {
	/// The kitty image id (`i=`), unique per attached image.
	pub id: u32,
	/// The PNG bytes, base64-encoded for direct (`t=d`) transmission.
	pub data: String,
	/// Pixel dimensions, parsed from the PNG header.
	pub px: UVec2,
}

/// The cell box a raster may occupy: the columns available on its line, and the
/// rows of the nearest scroll port it renders into (the viewport when nothing
/// clips).
///
/// The row bound is what keeps a raster placeable. Sized on aspect alone a
/// 1280x960 photo across 80 columns wants 30 rows, taller than any 24-row
/// window, so its box could never be shown whole — and a box that can never be
/// shown is a blank hole in the page, not a picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellBounds {
	pub cols: u32,
	pub rows: u32,
}

impl CellBounds {
	/// Bounds of `cols` x `rows`, each at least one cell so a degenerate
	/// (collapsed or zero-height) container still yields a placeable box.
	pub fn new(cols: u32, rows: u32) -> Self {
		Self {
			cols: cols.max(1),
			rows: rows.max(1),
		}
	}
}

impl KittyImage {
	/// The cell footprint within `bounds`: a nominal 10px column and the ~2:1
	/// cell aspect, preserving the raster's aspect ratio. The terminal scales the
	/// image to exactly this rect (`c=`/`r=`).
	pub fn cell_size(&self, bounds: CellBounds) -> UVec2 {
		const CELL_PX_WIDTH: u32 = 10;
		self.contain(self.px.x.div_ceil(CELL_PX_WIDTH).min(bounds.cols), bounds)
	}

	/// The cell footprint honoring explicit box dimensions: a missing axis
	/// derives from the raster's aspect, like a CSS replaced element with
	/// `width`/`height: auto`; with neither, [`cell_size`](Self::cell_size).
	pub fn cell_size_constrained(
		&self,
		width: Option<u32>,
		height: Option<u32>,
		bounds: CellBounds,
	) -> UVec2 {
		match (width, height) {
			// both axes authored: no aspect left to preserve, so the box is taken
			// as given. An oversized one is no longer a blank hole — the placement
			// pass draws whatever part of it the scroll port shows.
			(Some(cols), Some(rows)) => UVec2::new(cols.max(1), rows.max(1)),
			(Some(cols), None) => self.contain(cols, bounds),
			(None, Some(rows)) => {
				let rows = rows.max(1);
				// invert the 2:1 cell aspect: cols = rows * 2 * (px_w / px_h)
				let cols = self.cols_for(rows);
				// a tall raster (eg a `height: 70vh` hero on a narrow terminal)
				// derives a width wider than the available columns, and a height
				// past the scroll port is a box the port can never show whole.
				// Clamping one axis alone would squash the aspect, so either way
				// fall back to a width-driven fit: the raster stays aspect-correct
				// and inside the port, just shorter than the requested height.
				if cols > bounds.cols || rows > bounds.rows {
					self.contain(bounds.cols, bounds)
				} else {
					UVec2::new(cols, rows)
				}
			}
			(None, None) => self.cell_size(bounds),
		}
	}

	/// Contain a `cols`-wide box in `bounds`: rows follow the raster's aspect,
	/// and when they overflow the row bound the columns re-derive from it
	/// instead, so a tall raster shrinks to its scroll port rather than growing a
	/// box no window can place.
	fn contain(&self, cols: u32, bounds: CellBounds) -> UVec2 {
		let cols = cols.max(1);
		let rows = self.rows_for(cols);
		if rows <= bounds.rows {
			return UVec2::new(cols, rows);
		}
		let cols = self.cols_within(bounds.rows).min(cols);
		UVec2::new(cols, self.rows_for(cols))
	}

	/// Aspect-preserving rows for a `cols`-wide box, cells being ~2:1.
	fn rows_for(&self, cols: u32) -> u32 {
		(cols * self.px.y).div_ceil(self.px.x.max(1) * 2).max(1)
	}

	/// Aspect-preserving columns for a `rows`-tall box, rounded up so the box
	/// fills a requested height.
	fn cols_for(&self, rows: u32) -> u32 {
		(rows * 2 * self.px.x).div_ceil(self.px.y.max(1)).max(1)
	}

	/// Aspect-preserving columns for a `rows`-tall box, rounded down so
	/// [`rows_for`](Self::rows_for) of the result never exceeds `rows` — the
	/// containing direction, where overshooting by a single row is the whole
	/// failure the bound exists to prevent.
	fn cols_within(&self, rows: u32) -> u32 {
		(rows * 2 * self.px.x / self.px.y.max(1)).max(1)
	}
}

/// The element state the attach system sets on a raster-backed `<img>`, giving
/// it the terminal-gated block box (see [`default_element_rules`]).
pub(crate) fn graphics_state() -> ElementState {
	ElementState::Custom("graphics".into())
}

/// Pixel dimensions from a PNG header (`IHDR` width/height), or `None` when
/// the bytes are not a PNG.
#[cfg(any(all(feature = "tui", feature = "net"), test))]
pub(crate) fn png_dimensions(bytes: &[u8]) -> Option<UVec2> {
	(bytes.len() >= 24
		&& bytes.starts_with(b"\x89PNG\r\n\x1a\n")
		&& &bytes[12..16] == b"IHDR")
		.then(|| {
			UVec2::new(
				u32::from_be_bytes(bytes[16..20].try_into().unwrap()),
				u32::from_be_bytes(bytes[20..24].try_into().unwrap()),
			)
		})
}

// ── Detection ─────────────────────────────────────────────────────────────────

/// Whether a surface's terminal renders the kitty graphics protocol.
///
/// A *per-surface* component (one terminal per session), not a global resource:
/// over SSH the protocol is the *client's* capability, which the server process'
/// own `TERM` cannot report, so each session detects from the terminal it
/// actually talks to — a local [`StdioTerminal`] from the process env
/// ([`Default`]), an SSH session from its pty's terminal name
/// ([`from_term`](Self::from_term)). Absent or `enabled: false` keeps the
/// `[image]: alt` marker; insert one with `enabled: true` to force it on.
#[cfg(feature = "tui")]
#[derive(Debug, Clone, Component)]
pub struct KittyGraphicsSupport {
	pub enabled: bool,
}

#[cfg(feature = "tui")]
impl KittyGraphicsSupport {
	/// Detect support from a terminal-type name (a `TERM` value or an SSH pty's
	/// terminal): kitty and ghostty advertise the protocol in their term name, so
	/// a forwarded `xterm-kitty`/`xterm-ghostty` reports the client's capability.
	pub fn from_term(term: &str) -> Self {
		Self {
			enabled: term.contains("kitty") || term.contains("ghostty"),
		}
	}

	/// Detect from an SSH pty request: the forwarded terminal name advertises
	/// kitty/ghostty ([`from_term`](Self::from_term)), OR the client reports a
	/// non-zero pixel window size.
	///
	/// A kitty-graphics terminal (kitty, ghostty, WezTerm, iTerm2, foot) reports
	/// its pixel size in the pty request, where `TERM` is commonly flattened to
	/// `xterm-256color` over SSH (the server lacks the client's terminfo). A
	/// terminal that reports pixels without graphics support silently ignores the
	/// APC escapes (they are not echoed as garbage), so this errs toward enabling
	/// rich rendering for modern clients rather than forcing the alt marker.
	pub fn from_pty(term: &str, pixels: UVec2) -> Self {
		Self {
			enabled: Self::from_term(term).enabled
				|| (pixels.x > 0 && pixels.y > 0),
		}
	}
}

#[cfg(feature = "tui")]
impl Default for KittyGraphicsSupport {
	/// Detect from the local process environment, for a [`StdioTerminal`]: the
	/// `TERM` name plus the marker vars kitty and WezTerm export.
	fn default() -> Self {
		let term = env_ext::var("TERM").unwrap_or_default();
		let enabled = Self::from_term(&term).enabled
			|| env_ext::var("KITTY_WINDOW_ID").is_ok()
			|| env_ext::var("TERM_PROGRAM").is_ok_and(|prog| prog == "WezTerm");
		Self { enabled }
	}
}

// ── Attach ────────────────────────────────────────────────────────────────────

/// Marks an `<img>` whose `src` could not back a raster (a failed or non-2xx
/// fetch, not a decodable image), so the attach system tries it exactly once and
/// the marker + error fallback presents it. Carries the failure message, rendered
/// alongside the `[image]: alt` marker by [`render_image_errors`].
#[cfg(feature = "tui")]
#[derive(Debug, Clone, Component)]
pub struct KittyImageUnavailable {
	/// The failure message, rendered in the material [`Error`] box.
	pub error: SmolStr,
}

/// Marks an unavailable `<img>` whose alt + error fallback has been spawned, so
/// [`render_image_errors`] builds it exactly once.
#[cfg(feature = "tui")]
#[derive(Debug, Clone, Copy, Component)]
pub struct KittyErrorShown;

/// Marks an `<img>` whose remote `src` is being fetched, so exactly one fetch
/// is in flight. The alt marker presents until the bytes arrive.
#[cfg(feature = "tui")]
#[derive(Debug, Clone, Copy, Component)]
pub struct KittyImageLoading;

/// ECS system: back each new `<img>` with a [`KittyImage`] when the terminal
/// supports graphics, by fetching its `src` over HTTP (`net` feature) and
/// attaching on arrival. An absolute `http(s)://` fetches directly; a site-rooted
/// `/assets/…` loops back to our own canonical server, exactly as a browser
/// resolves it against the document origin. Without the `net` feature there is no
/// transport, so an `<img>` is simply marked unavailable.
#[cfg(feature = "tui")]
pub(crate) fn attach_kitty_images(
	mut placements: ResMut<KittyPlacements>,
	elements: ElementQuery,
	surfaces: SurfaceQuery,
	support: Query<&KittyGraphicsSupport>,
	unvisited: Query<
		(),
		(
			With<Element>,
			Without<KittyImage>,
			Without<KittyImageLoading>,
			Without<KittyImageUnavailable>,
		),
	>,
	mut commands: Commands,
) {
	// `placements` allocates raster ids only on the `net` fetch path.
	#[cfg(not(feature = "net"))]
	let _ = &mut placements;
	for view in elements.iter() {
		if view.tag() != "img" || !unvisited.contains(view.entity) {
			continue;
		}
		// the img's own surface must render the graphics protocol; resolve it
		// across the page's portal transclusion (route content has no `ChildOf`
		// path to the page root that carries the surface). A session whose terminal
		// has no graphics keeps the `[image]: alt` marker.
		let supported = surfaces
			.surface_of(view.entity)
			.and_then(|surface| support.get(surface).ok())
			.is_some_and(|support| support.enabled);
		if !supported {
			continue;
		}
		let src = view.attribute_string("src");
		if src.is_empty() {
			continue;
		}
		// fetch the `src` over HTTP in the background and attach on arrival. An
		// authority-less `/assets/…` loops back to the canonical server (Part A).
		#[cfg(feature = "net")]
		{
			let id = placements.alloc_id();
			let src = src.clone();
			commands.entity(view.entity).insert(KittyImageLoading);
			commands
				.entity(view.entity)
				.queue_async(move |entity| fetch_remote(entity, src, id));
		}
		// no transport compiled in: nothing can load the image.
		#[cfg(not(feature = "net"))]
		commands.entity(view.entity).insert(KittyImageUnavailable {
			error: "the 'net' feature is required to load images".into(),
		});
	}
}

/// Insert the raster and the `graphics` element state driving its block box,
/// merging into any states the element already carries (eg hover).
// the attach path is reached only by the `net` fetch (and the test harness that
// attaches a raster directly); without either there is nothing to attach.
#[cfg(all(feature = "tui", any(feature = "net", test)))]
fn attach_image(mut entity: EntityWorldMut, image: KittyImage) {
	entity.insert(image);
	match entity.get_mut::<ElementStateMap>() {
		Some(mut map) => {
			map.insert(graphics_state());
		}
		None => {
			entity.insert(ElementStateMap::with(graphics_state()));
		}
	}
}

/// System: render an unavailable `<img>`'s error once, alongside its existing
/// `[image]: alt` marker. The failed `<img>` keeps its [`Marker`] (the
/// `[image]: alt` text, rendered as the gutter once it has children), and this
/// spawns the material [`Error`] box carrying the failure as a child, so a failed
/// image shows both what it was and why it could not load.
#[cfg(feature = "tui")]
pub(crate) fn render_image_errors(
	unavailable: Query<
		(Entity, &KittyImageUnavailable),
		Without<KittyErrorShown>,
	>,
	mut commands: Commands,
) {
	for (entity, image) in unavailable.iter() {
		let error = image.error.clone();
		commands
			.entity(entity)
			.insert(KittyErrorShown)
			.with_children(|parent| {
				parent.spawn(rsx! { <Error>{error}</Error> });
			});
	}
}

/// PNG bytes for an image: PNG input passes through; an SVG is rasterised to
/// PNG ([`svg_to_png`]); any other format the `image` decoder understands (eg
/// JPEG) is decoded to RGBA and re-encoded to PNG. `None` when the bytes are
/// not a decodable image.
#[cfg(all(feature = "tui", any(feature = "net", test)))]
fn to_png_bytes(bytes: Vec<u8>) -> Option<Vec<u8>> {
	if png_dimensions(&bytes).is_some() {
		return Some(bytes);
	}
	if is_svg(&bytes) {
		return svg_to_png(&bytes);
	}
	let image = image::load_from_memory(&bytes).ok()?;
	let mut png = std::io::Cursor::new(Vec::new());
	image
		.write_to(&mut png, image::ImageFormat::Png)
		.ok()
		.map(|_| png.into_inner())
}

/// Whether `bytes` look like an SVG: valid-ish UTF-8 text whose first non-blank
/// byte opens a tag and which contains an `<svg` within the sniffed head. PNGs
/// are returned before this is reached, and the other raster formats are binary
/// (JPEG `\xff\xd8`, GIF `GIF8`, WebP `RIFF`), so none of them misfire here.
#[cfg(all(feature = "tui", any(feature = "net", test)))]
fn is_svg(bytes: &[u8]) -> bool {
	let head = &bytes[..bytes.len().min(1024)];
	let text = String::from_utf8_lossy(head);
	let trimmed = text.trim_start_matches('\u{feff}').trim_start();
	trimmed.starts_with('<') && text.contains("<svg")
}

/// System fonts for SVG `<text>`, loaded once. `load_system_fonts` walks the
/// platform font directories, so the database is cached behind a `OnceLock`
/// rather than rebuilt for every rasterised image.
#[cfg(all(feature = "tui", any(feature = "net", test)))]
fn svg_fontdb() -> std::sync::Arc<resvg::usvg::fontdb::Database> {
	use std::sync::Arc;
	use std::sync::OnceLock;
	static FONTDB: OnceLock<Arc<resvg::usvg::fontdb::Database>> =
		OnceLock::new();
	FONTDB
		.get_or_init(|| {
			let mut db = resvg::usvg::fontdb::Database::new();
			db.load_system_fonts();
			Arc::new(db)
		})
		.clone()
}

/// Rasterise an SVG to PNG bytes, or `None` when the bytes do not parse as an
/// SVG. Rendered at 2× and left for the terminal to downscale, so text and thin
/// strokes stay crisp; the target is clamped so a pathological `viewBox` cannot
/// allocate an unbounded pixmap. The figure's own colours are honoured verbatim
/// — a deck SVG authored in the site palette therefore rasterises on-theme (the
/// palette lives in the SVG, the single surface a re-theme would touch).
#[cfg(all(feature = "tui", any(feature = "net", test)))]
fn svg_to_png(bytes: &[u8]) -> Option<Vec<u8>> {
	use resvg::tiny_skia;
	use resvg::usvg;

	let options = usvg::Options {
		fontdb: svg_fontdb(),
		..Default::default()
	};
	let tree = usvg::Tree::from_data(bytes, &options).ok()?;

	const SCALE: f32 = 2.0;
	const MAX_PX: u32 = 4096;
	let size = tree.size();
	let width = ((size.width() * SCALE).ceil() as u32).clamp(1, MAX_PX);
	let height = ((size.height() * SCALE).ceil() as u32).clamp(1, MAX_PX);

	let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
	resvg::render(
		&tree,
		tiny_skia::Transform::from_scale(SCALE, SCALE),
		&mut pixmap.as_mut(),
	);
	pixmap.encode_png().ok()
}

/// Background fetch for an image `src` over HTTP: attach the raster on arrival,
/// or mark the element unavailable carrying the failure so [`render_image_errors`]
/// shows the alt marker plus the styled error.
#[cfg(all(feature = "tui", feature = "net"))]
async fn fetch_remote(entity: AsyncEntity, src: String, id: u32) -> Result {
	// decode + rasterise + encode on the blocking pool, never inline. Beet runs
	// bevy single-threaded, so every detached task shares the one world thread:
	// an inline `image::load_from_memory`, a `resvg::render` of up to 4096², or
	// the one-time `fontdb::load_system_fonts()` directory walk freezes every
	// other connection for as long as it runs. On a throttled 2-vCPU box that is
	// a multi-second stall of the whole server per `<img>`.
	let loaded = match fetch_image_bytes(&src).await {
		Ok(bytes) => {
			blocking::unblock(move || {
				to_png_bytes(bytes).and_then(encode_png).ok_or_else(|| {
					bevyhow!("response is not a decodable image")
				})
			})
			.await
		}
		Err(err) => Err(err),
	};
	// each failure mode warns the src so a no-port error reads differently from a
	// refused connection, a non-2xx, or a decode error, instead of a silent blank.
	if let Err(err) = &loaded {
		warn!("img src {src:?}: {err}");
	}
	entity
		.with(move |mut entity| {
			entity.remove::<KittyImageLoading>();
			match loaded {
				Ok((data, px)) => {
					attach_image(entity, KittyImage { id, data, px });
				}
				Err(err) => {
					entity.insert(KittyImageUnavailable {
						error: err.to_string().into(),
					});
				}
			}
		})
		.await
}

/// The raw response bytes for an image `src`, fetched over HTTP, or the precise
/// failure: a refused/failed send (incl the no-port error when no canonical
/// server is up), a non-2xx status, or a body-read error.
#[cfg(all(feature = "tui", feature = "net"))]
async fn fetch_image_bytes(src: &str) -> Result<Vec<u8>> {
	use beet_net::prelude::*;
	// no `Accept` constraint: the server returns the stored file (jpg/png/…) and
	// `to_png_bytes` decodes it; pinning `Accept: png` would reject a jpg asset. An
	// authority-less `/assets/…` loopback-rewrites in `send` (Part A).
	Request::get(src)
		.send()
		.await?
		.into_result()
		.await?
		.bytes_vec()
		.await
}

/// Validate and base64-encode PNG bytes, with their parsed dimensions.
#[cfg(all(feature = "tui", any(feature = "net", test)))]
fn encode_png(bytes: Vec<u8>) -> Option<(String, UVec2)> {
	use base64::Engine;
	let px = png_dimensions(&bytes)?;
	let data = base64::engine::general_purpose::STANDARD.encode(&bytes);
	Some((data, px))
}

// ── Placement ─────────────────────────────────────────────────────────────────

/// Per-terminal kitty placement state: what is currently drawn where, so the
/// emission diffs placements exactly as the cell renderer diffs cells.
#[cfg(feature = "tui")]
#[derive(Debug, Default, Resource)]
pub struct KittyPlacements {
	next_id: u32,
	terminals: HashMap<Entity, TerminalPlacements>,
}

#[cfg(feature = "tui")]
#[derive(Debug, Default)]
struct TerminalPlacements {
	/// The viewport these placements were computed against; a change (resize)
	/// invalidates them all.
	viewport: UVec2,
	/// Image ids whose payload this terminal has already received.
	transmitted: HashSet<u32>,
	/// The placed on-screen rect of each image entity.
	placed: HashMap<Entity, PlacedImage>,
}

/// One image's on-screen placement: where it is drawn and which part of the
/// raster is drawn there.
///
/// A fully visible box draws the whole raster (`crop: None`); a box the scroll
/// port only partly shows draws the matching source rect into the visible cells,
/// so the picture slides under its port instead of disappearing the moment an
/// edge crosses it. The crop is part of the placement's identity, so scrolling
/// re-places through the same diff a move does.
#[cfg(feature = "tui")]
#[derive(Debug, Clone, Copy, PartialEq)]
struct PlacedImage {
	id: u32,
	pos: UVec2,
	cells: UVec2,
	/// The source rect in raster pixels, `None` when the whole raster is drawn.
	crop: Option<URect>,
}

#[cfg(feature = "tui")]
impl PlacedImage {
	/// The placement drawing `rect`'s `visible` portion of a `px`-pixel raster.
	///
	/// `visible` is a sub-rect of `rect`, so the source rect is that same
	/// fraction of the pixels: the visible cells keep exactly the scale a whole
	/// placement would have drawn at, and nothing shifts as the crop grows.
	fn new(id: u32, px: UVec2, rect: IRect, visible: IRect) -> Self {
		let span = UVec2::new(rect.width() as u32, rect.height() as u32);
		let crop = (visible != rect).then(|| {
			let fraction = |axis: u32, offset: i32, span: u32| {
				(axis as u64 * offset as u64 / span.max(1) as u64) as u32
			};
			let min = UVec2::new(
				fraction(px.x, visible.min.x - rect.min.x, span.x),
				fraction(px.y, visible.min.y - rect.min.y, span.y),
			);
			let max = UVec2::new(
				fraction(px.x, visible.max.x - rect.min.x, span.x),
				fraction(px.y, visible.max.y - rect.min.y, span.y),
			);
			// a sliver of a cell can round the source rect flat; the protocol
			// needs at least one pixel on each axis to have something to scale.
			URect {
				min,
				max: max.max(min + UVec2::ONE),
			}
		});
		Self {
			id,
			pos: UVec2::new(visible.min.x as u32, visible.min.y as u32),
			cells: UVec2::new(visible.width() as u32, visible.height() as u32),
			crop,
		}
	}
}

#[cfg(feature = "tui")]
impl KittyPlacements {
	/// The next unused kitty image id.
	pub fn alloc_id(&mut self) -> u32 {
		self.next_id += 1;
		self.next_id
	}
}

/// Observer: drop a gone terminal's placement state.
///
/// The map is keyed by surface, and a multi-tenant server's surfaces come and go
/// with its clients (one per SSH session), so without this it grows an entry —
/// with every id it ever transmitted — for every client that ever connected.
/// Registered by [`CharcellPlugin`](crate::prelude::CharcellPlugin) beside the
/// resource.
#[cfg(feature = "tui")]
pub(crate) fn clear_kitty_placements(
	ev: On<Remove, Terminal>,
	mut placements: ResMut<KittyPlacements>,
) {
	placements.terminals.remove(&ev.entity);
}

/// ECS system: transmit and (re)place each visible [`KittyImage`] after the
/// cell renderer has drawn, diffing against [`KittyPlacements`] so escapes are
/// only emitted when an image appears, moves, resizes, or disappears.
#[cfg(feature = "tui")]
pub(crate) fn place_kitty_images(
	mut placements: ResMut<KittyPlacements>,
	mut terminals: Query<(
		Entity,
		&mut Terminal,
		&DoubleBuffer,
		Option<&KittyGraphicsSupport>,
	)>,
	charcell: CharcellQuery,
	tree: CharcellTree,
	images: Query<&KittyImage>,
) -> Result {
	for (root, mut terminal, buffer, support) in terminals.iter_mut() {
		// only place into a surface whose terminal renders the graphics protocol.
		if !support.is_some_and(|support| support.enabled) {
			continue;
		}
		let viewport = buffer.size();
		let state = placements.terminals.entry(root).or_default();
		let writer = terminal.writer_mut();

		// a resize reallocated the screen: drop every placement and re-send each
		// image from scratch. Terminals discard image data when the screen is
		// cleared/reflowed on resize (ghostty does), so the transmit cache is
		// cleared too — the next placement retransmits the bytes rather than
		// placing a now-absent image and leaving a blank.
		if state.viewport != viewport {
			if !state.placed.is_empty() {
				write_delete_all(writer)?;
			}
			state.placed.clear();
			state.transmitted.clear();
			state.viewport = viewport;
		}

		let desired =
			desired_placements(root, viewport, &charcell, &tree, &images);

		// remove placements for images gone from the frame
		let stale = state
			.placed
			.iter()
			.filter(|(entity, _)| !desired.contains_key(*entity))
			.map(|(&entity, &placed)| (entity, placed))
			.collect::<Vec<_>>();
		for (entity, placed) in stale {
			write_delete(writer, placed.id)?;
			state.placed.remove(&entity);
		}

		// transmit new payloads, place new/moved images
		for (entity, placed) in desired {
			if state.placed.get(&entity) == Some(&placed) {
				continue;
			}
			if let Some(previous) = state.placed.get(&entity) {
				write_delete(writer, previous.id)?;
			}
			if state.transmitted.insert(placed.id) {
				let image = images
					.iter()
					.find(|image| image.id == placed.id)
					.ok_or_else(|| {
					bevyhow!("missing image {}", placed.id)
				})?;
				write_transmit(writer, placed.id, &image.data)?;
			}
			write_place(writer, &placed)?;
			state.placed.insert(entity, placed);
		}
	}
	Ok(())
}

/// The visible images under `root` and the placement each should occupy,
/// through the same scroll translation and clip the paint applied.
///
/// An image the clip only partly shows is placed cropped to that intersection,
/// not dropped: a scrolled picture slides under its port a row at a time instead
/// of popping in and out at the edges. An image the clip excludes entirely is
/// legitimately absent (scrolled away), and the sizing pass has already
/// contained every auto-sized box within its port, so "never placeable" is not a
/// state a raster can reach.
#[cfg(feature = "tui")]
fn desired_placements(
	root: Entity,
	viewport: UVec2,
	charcell: &CharcellQuery,
	tree: &CharcellTree,
	images: &Query<&KittyImage>,
) -> HashMap<Entity, PlacedImage> {
	let ordered = tree.pre_order(root);
	let contexts = resolve_contexts(root, &ordered, charcell, tree, viewport);
	let screen = IRect::new(0, 0, viewport.x as i32, viewport.y as i32);
	let mut desired = HashMap::default();
	for &entity in &ordered {
		let Ok(image) = images.get(entity) else {
			continue;
		};
		let Ok(node) = charcell.unresolved_node(entity) else {
			continue;
		};
		let cx = contexts.get(&entity).copied().unwrap_or_default();
		let rect = BoxModel::from_node(&node, viewport)
			.content_rect(translate_rect(node.layout_rect(), cx.offset));
		if rect.width() <= 0 || rect.height() <= 0 {
			continue;
		}
		// the part of the box the overflow clip and the screen both show
		let visible = screen.intersect(cx.clip.intersect(rect));
		if visible.width() <= 0 || visible.height() <= 0 {
			continue;
		}
		desired.insert(
			entity,
			PlacedImage::new(image.id, image.px, rect, visible),
		);
	}
	desired
}

// ── Escape emission ───────────────────────────────────────────────────────────

/// Payload bytes per transmission chunk, the protocol's required maximum.
#[cfg(feature = "tui")]
const CHUNK: usize = 4096;

/// Transmit a base64 PNG payload (`a=t`), chunked at [`CHUNK`] bytes.
#[cfg(feature = "tui")]
fn write_transmit(
	w: &mut (impl Write + ?Sized),
	id: u32,
	data: &str,
) -> Result {
	let mut chunks = data.as_bytes().chunks(CHUNK).peekable();
	let mut first = true;
	while let Some(chunk) = chunks.next() {
		let more = chunks.peek().is_some() as u8;
		match (first, more) {
			// a single-chunk payload omits the continuation key entirely
			(true, 0) => write!(w, "\x1b_Ga=t,f=100,q=2,i={id};")?,
			(true, _) => write!(w, "\x1b_Ga=t,f=100,q=2,i={id},m=1;")?,
			(false, more) => write!(w, "\x1b_Gm={more};")?,
		}
		w.write_all(chunk)?;
		w.write_all(b"\x1b\\")?;
		first = false;
	}
	Ok(())
}

/// Place image `id` over the given cell rect (`a=p`), scaling to fit and
/// leaving the cursor where it was.
///
/// A cropped placement names its source rect in raster pixels (`x=`/`y=`/`w=`/
/// `h=`); a whole one omits the keys, so a fully visible image emits exactly the
/// escape it always did.
#[cfg(feature = "tui")]
fn write_place(w: &mut (impl Write + ?Sized), placed: &PlacedImage) -> Result {
	escape::cursor_goto(&mut &mut *w, placed.pos)?;
	write!(w, "\x1b_Ga=p,i={}", placed.id)?;
	if let Some(crop) = placed.crop {
		write!(
			w,
			",x={},y={},w={},h={}",
			crop.min.x,
			crop.min.y,
			crop.width(),
			crop.height()
		)?;
	}
	write!(
		w,
		",c={},r={},q=2,C=1\x1b\\",
		placed.cells.x, placed.cells.y
	)?;
	Ok(())
}

/// Delete the placements of image `id` (`a=d,d=i`), retaining its data so a
/// later re-place is cheap.
#[cfg(feature = "tui")]
fn write_delete(w: &mut (impl Write + ?Sized), id: u32) -> Result {
	write!(w, "\x1b_Ga=d,d=i,i={id},q=2\x1b\\")?;
	Ok(())
}

/// Delete every visible placement (`a=d,d=a`), used on resize.
#[cfg(feature = "tui")]
fn write_delete_all(w: &mut (impl Write + ?Sized)) -> Result {
	write!(w, "\x1b_Ga=d,d=a,q=2\x1b\\")?;
	Ok(())
}

#[cfg(test)]
mod test {
	use super::*;
	// the style types shadow same-named `bevy_ui` ones leaking through the
	// preludes when `bevy_default` is co-enabled.
	#[cfg(feature = "tui")]
	use crate::input::ScrollPosition;
	#[cfg(feature = "tui")]
	use crate::style::Length;
	#[cfg(feature = "tui")]
	use crate::style::Overflow;
	#[cfg(feature = "tui")]
	use crate::style::common_props;
	#[cfg(feature = "tui")]
	use bevy::math::IVec2;

	/// Minimal PNG header bytes for a `width`x`height` image: enough for the
	/// loader (magic + IHDR dimensions); the terminal never sees it in tests.
	fn png_bytes(width: u32, height: u32) -> Vec<u8> {
		let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
		bytes.extend(13u32.to_be_bytes());
		bytes.extend(b"IHDR");
		bytes.extend(width.to_be_bytes());
		bytes.extend(height.to_be_bytes());
		bytes.extend([8, 6, 0, 0, 0]);
		bytes
	}

	#[beet_core::test]
	fn parses_png_dimensions() {
		png_dimensions(&png_bytes(640, 480))
			.xpect_eq(Some(UVec2::new(640, 480)));
		png_dimensions(b"not a png").xpect_eq(None);
	}

	/// A raster of the given pixel size, the only field the sizing math reads.
	fn sized_image(px: UVec2) -> KittyImage {
		KittyImage {
			id: 1,
			data: String::new(),
			px,
		}
	}

	/// The cell box preserves aspect through the ~2:1 cell shape and clamps to
	/// the available columns.
	#[beet_core::test]
	fn cell_size_preserves_aspect() {
		let image = sized_image(UVec2::new(200, 100));
		// 200px / 10 = 20 cols; rows = 20 * (100/200) / 2 = 5
		image
			.cell_size(CellBounds::new(80, 24))
			.xpect_eq(UVec2::new(20, 5));
		// clamped to 10 cols, rows follow the aspect
		image
			.cell_size(CellBounds::new(10, 24))
			.xpect_eq(UVec2::new(10, 3));
	}

	/// THE sizing regression: the blog post's 1280x960 photo. Sized on the column
	/// bound alone it wants 30 rows — taller than any 24-row window, a box that
	/// can never be placed — so the row bound re-derives the columns and the
	/// picture fits its port with the aspect intact.
	#[beet_core::test]
	fn cell_size_contains_within_the_row_bound() {
		let image = sized_image(UVec2::new(1280, 960));
		// 128 cols of raster clamp to 80, whose aspect rows (30) overflow the
		// window: the columns re-derive from the 24 rows instead.
		image
			.cell_size(CellBounds::new(80, 24))
			.xpect_eq(UVec2::new(64, 24));
		// an 8-row scroll port inside that window bounds it further, still on
		// aspect (a 21-col box is exactly 8 rows tall).
		image
			.cell_size(CellBounds::new(80, 8))
			.xpect_eq(UVec2::new(21, 8));
		// the derived box never overshoots the bound it was contained to
		image.rows_for(21).xpect_eq(8);
	}

	/// An explicit height derives the width from the aspect; when that width fits
	/// the columns and the height fits the port it is honored, and when either
	/// overflows the box falls back to a width-driven fit (aspect-correct,
	/// shorter) rather than squashing.
	#[beet_core::test]
	fn cell_size_constrained_contains_tall_height() {
		// 16:9 raster, the deck's UHD hero shape.
		let image = sized_image(UVec2::new(3840, 2160));
		// height 10 rows -> width = 10 * 2 * 3840/2160 = 35.6 -> 36 cols; fits in
		// 80, so the requested height stands.
		image
			.cell_size_constrained(None, Some(10), CellBounds::new(80, 24))
			.xpect_eq(UVec2::new(36, 10));
		// same height on a 30-col terminal: 36 cols would overflow, so fit to
		// width instead (rows follow the aspect, shorter than the requested 10).
		image
			.cell_size_constrained(None, Some(10), CellBounds::new(30, 24))
			.xpect_eq(UVec2::new(30, image.rows_for(30)));
		// a `70vh` hero in a 10-row scroll port: the height itself overflows, so
		// the same width-driven fit contains it.
		image
			.cell_size_constrained(None, Some(20), CellBounds::new(80, 10))
			.xpect_eq(UVec2::new(35, 10));
		// an explicit width whose aspect rows overflow the port narrows too
		image
			.cell_size_constrained(Some(80), None, CellBounds::new(80, 10))
			.xpect_eq(UVec2::new(35, 10));
	}

	// the live-terminal cases drive the `TestHost`/`KittyGraphicsSupport`
	// machinery, both `tui`-gated; default-feature builds skip them.
	#[cfg(feature = "tui")]
	use crate::render::charcell::test_host::TestHost;

	/// A `size`-cell host with graphics forced on, showing `content` under
	/// `rules`, its `<img>` backed by a `px`-pixel [`KittyImage`].
	///
	/// The raster is attached directly rather than fetched: the fetch path is
	/// `net`-gated and needs a server, and sizing/placement are independent of how
	/// the bytes arrived.
	#[cfg(feature = "tui")]
	fn image_host_with(
		size: UVec2,
		px: UVec2,
		rules: Vec<Rule>,
		content: impl Bundle,
	) -> TestHost {
		let mut host = TestHost::sized(size);
		host.app
			.world_mut()
			.entity_mut(host.host)
			.insert(KittyGraphicsSupport { enabled: true });
		// rules must be registered before the content resolves its styles
		host.app
			.world_mut()
			.get_resource_or_init::<RuleSet>()
			.extend_rules(rules);
		host.spawn_content(content);
		let (data, px) = encode_png(png_bytes(px.x, px.y)).expect("valid png");
		let world = host.app.world_mut();
		let img = world
			.query_filtered::<(Entity, &Element), With<Element>>()
			.iter(world)
			.find(|(_, element)| element.tag() == "img")
			.map(|(entity, _)| entity)
			.expect("img element");
		attach_image(world.entity_mut(img), KittyImage { id: 1, data, px });
		host.step();
		host
	}

	/// [`image_host_with`] showing a lone `<img>` in a `size`-cell window.
	#[cfg(feature = "tui")]
	fn image_host_sized(size: UVec2, width: u32, height: u32) -> TestHost {
		image_host_with(size, UVec2::new(width, height), vec![], rsx! {
			<div><img src="x.png" alt="a test image"/></div>
		})
	}

	/// [`image_host_sized`] at the small default snapshot viewport.
	#[cfg(feature = "tui")]
	fn image_host(width: u32, height: u32) -> TestHost {
		image_host_sized(UVec2::new(40, 14), width, height)
	}

	/// A `height`-row scroll port under `class`, the shape every port test drives.
	#[cfg(feature = "tui")]
	fn scroll_port(class: &str, height: f32) -> Rule {
		Rule::class(class)
			.with_value(common_props::Height, Length::Rem(height))
			.with_value(common_props::OverflowYProp, Overflow::Scroll)
	}

	/// The `<img>`'s laid-out rect, for driving a scroll to a known edge of it.
	#[cfg(feature = "tui")]
	fn image_rect(host: &mut TestHost) -> IRect {
		host.app
			.world_mut()
			.query_filtered::<&LayoutRect, With<KittyImage>>()
			.single(host.app.world())
			.unwrap()
			.0
	}

	/// Scroll the tree's scroll container to `rows` and repaint.
	#[cfg(feature = "tui")]
	fn scroll_to(host: &mut TestHost, rows: i32) {
		let world = host.app.world_mut();
		let port = world
			.query_filtered::<Entity, With<ScrollPosition>>()
			.single(world)
			.unwrap();
		world
			.entity_mut(port)
			.insert(ScrollPosition::new(IVec2::new(0, rows)));
		host.step();
	}

	/// The keys of the first `a=p` placement escape in an emitted frame: the
	/// `c`/`r` cell box, and `x`/`y`/`w`/`h` when the placement is cropped.
	#[cfg(feature = "tui")]
	fn placement_keys(host: &mut TestHost) -> HashMap<String, u32> {
		String::from_utf8_lossy(&host.frame_ansi())
			.split("\u{1b}_G")
			.find(|escape| escape.starts_with("a=p"))
			.expect("a placement escape")
			.split('\u{1b}')
			.next()
			.unwrap()
			.split(',')
			.filter_map(|pair| pair.split_once('='))
			.filter_map(|(key, value)| {
				value.parse().ok().map(|value| (key.to_string(), value))
			})
			.collect()
	}

	/// A supported terminal transmits the PNG once and places it at its
	/// laid-out cell rect; the alt-text fallback is not painted.
	#[cfg(feature = "tui")]
	#[beet_core::test]
	fn transmits_and_places_image() {
		let mut host = image_host(100, 40);
		let out = String::from_utf8_lossy(&host.frame_ansi()).into_owned();
		// transmitted as direct PNG data with the allocated id
		out.as_str()
			.xpect_contains("\u{1b}_Ga=t,f=100,q=2,i=1;")
			// placed over the 10x2 cell box (100px/10, aspect 40/100 over 2:1 cells)
			.xpect_contains("a=p,i=1,c=10,r=2,q=2,C=1");
		host.frame_plain().xnot().xpect_contains("[image]");
		// steady state re-emits nothing
		host.step();
		String::from_utf8_lossy(&host.frame_ansi())
			.into_owned()
			.xnot()
			.xpect_contains("\u{1b}_G");
	}

	/// THE default-window sizing regression: the blog post's 1280x960 photo in a
	/// standard 80x24 terminal. Sized on the column bound alone its box is 30
	/// rows, so the placement pass could never show it whole and the page held a
	/// blank hole; contained to the window it places, whole and on aspect.
	#[cfg(feature = "tui")]
	#[beet_core::test]
	fn tall_raster_fits_the_default_window() {
		let mut host = image_host_sized(UVec2::new(80, 24), 1280, 960);
		let keys = placement_keys(&mut host);
		keys["r"].xpect_less_or_equal_to(24);
		// the whole raster, uncropped, still on the raster's own aspect
		keys.contains_key("w").xpect_false();
		sized_image(UVec2::new(1280, 960))
			.rows_for(keys["c"])
			.xpect_eq(keys["r"]);
	}

	/// The bound is the nearest scroll port, not the window: nested ports, panes
	/// and sidebars are the general case, so a raster is contained by whichever
	/// clip it actually renders into.
	#[cfg(feature = "tui")]
	#[beet_core::test]
	fn raster_bounds_to_its_nested_scroll_port() {
		let mut host = image_host_with(
			UVec2::new(80, 24),
			UVec2::new(1280, 960),
			vec![scroll_port("outer", 12.), scroll_port("inner", 6.)],
			rsx! {
				<div class="outer">
					<div class="inner">
						<img src="x.png" alt="a test image"/>
					</div>
				</div>
			},
		);
		// bounded by the inner 6-row port, not the 12-row one or the 24-row window
		placement_keys(&mut host)["r"].xpect_less_or_equal_to(6);
	}

	/// A raster the scroll port only partly shows places its visible portion with
	/// a source crop, and scrolling moves that crop rather than dropping the
	/// picture — which is what made an image pop in and out at the port edges.
	#[cfg(feature = "tui")]
	#[beet_core::test]
	fn scrolled_raster_places_a_moving_crop() {
		let mut host = image_host_with(
			UVec2::new(40, 10),
			UVec2::new(200, 100),
			vec![scroll_port("port", 10.)],
			rsx! {
				<div class="port">
					<pre>"a\nb\nc\nd\ne\nf\ng\nh"</pre>
					<img src="x.png" alt="a test image"/>
					<pre>"i\nj\nk\nl\nm\nn\no\np"</pre>
				</div>
			},
		);
		// unscrolled the image straddles the port's *bottom* edge: its top rows
		// draw, from the matching top source pixels.
		let bottom = placement_keys(&mut host);
		bottom["y"].xpect_eq(0);
		bottom["h"].xpect_less_than(100);

		// scrolled flush to the port top the whole picture draws, uncropped.
		let rect = image_rect(&mut host);
		scroll_to(&mut host, rect.min.y);
		let whole = placement_keys(&mut host);
		whole.contains_key("y").xpect_false();
		let rows = whole["r"];
		bottom["r"].xpect_less_than(rows);

		// two rows further it straddles the *top* edge instead: the crop has moved
		// down the raster and the picture is still placed, not dropped.
		scroll_to(&mut host, rect.min.y + 2);
		let top = placement_keys(&mut host);
		top["r"].xpect_eq(rows - 2);
		top["y"].xpect_greater_than(bottom["y"]);
		top["h"].xpect_eq(100 - top["y"]);
	}

	/// No window size leaves a raster as a reserved blank with neither a picture
	/// nor an `[image]: alt` marker: the sizing contains the box to its port and
	/// the placement crops whatever the port shows.
	#[cfg(feature = "tui")]
	#[beet_core::test]
	fn every_viewport_places_the_raster() {
		for size in [
			UVec2::new(80, 24),
			UVec2::new(60, 40),
			UVec2::new(40, 14),
			UVec2::new(30, 10),
			UVec2::new(120, 30),
			UVec2::new(20, 6),
		] {
			let mut host = image_host_sized(size, 1280, 960);
			let keys = placement_keys(&mut host);
			keys["r"].xpect_greater_or_equal_to(1);
			keys["c"].xpect_greater_or_equal_to(1);
			host.frame_plain().xnot().xpect_contains("[image]");
		}
	}

	/// A non-PNG image (a JPEG) is decoded and re-encoded to PNG so the kitty
	/// `f=100` transmit handles it, with its dimensions preserved.
	#[cfg(all(feature = "tui", not(target_arch = "wasm32")))]
	#[beet_core::test]
	fn decodes_jpeg_image() {
		// a real 8x6 JPEG
		let jpeg = {
			let img = image::DynamicImage::ImageRgb8(
				image::RgbImage::from_pixel(8, 6, image::Rgb([200, 100, 50])),
			);
			let mut buf = std::io::Cursor::new(Vec::new());
			img.write_to(&mut buf, image::ImageFormat::Jpeg).unwrap();
			buf.into_inner()
		};
		// decoded + re-encoded to a valid PNG of the same dimensions
		to_png_bytes(jpeg)
			.and_then(encode_png)
			.unwrap()
			.1
			.xpect_eq(UVec2::new(8, 6));
	}

	/// Removing the image deletes its placement; a resize deletes all visible
	/// placements and re-sends each image (data + placement) from scratch, since a
	/// terminal may discard image data when the screen reflows on resize.
	#[cfg(feature = "tui")]
	#[beet_core::test]
	fn removal_and_resize_replace_placements() {
		let mut host = image_host(100, 40);
		host.frame_ansi();
		// resize: every placement is dropped, then each image is retransmitted and
		// re-placed.
		host.resize(UVec2::new(50, 16));
		host.step();
		let resized = String::from_utf8_lossy(&host.frame_ansi()).into_owned();
		resized
			.as_str()
			.xpect_contains("a=d,d=a,q=2")
			.xpect_contains("a=t,f=100,q=2,i=1")
			.xpect_contains("a=p,i=1");

		// despawning the img deletes its placement
		let img = host
			.app
			.world_mut()
			.query_filtered::<Entity, With<KittyImage>>()
			.single(host.app.world())
			.unwrap();
		host.app.world_mut().entity_mut(img).despawn();
		host.step();
		String::from_utf8_lossy(&host.frame_ansi())
			.into_owned()
			.xpect_contains("a=d,d=i,i=1,q=2");
	}

	/// Regression: a gone terminal's placements go with it. The state is keyed by
	/// surface in a resource, and a multi-tenant server's surfaces close with their
	/// clients (one per SSH session), so a stale entry per client — each holding
	/// every image id it was sent — outlives every session that ever connected.
	#[cfg(feature = "tui")]
	#[beet_core::test]
	fn despawned_terminal_drops_its_placements() {
		let mut host = image_host(100, 40);
		host.frame_ansi();
		let tracked = |host: &TestHost| {
			host.app
				.world()
				.resource::<KittyPlacements>()
				.terminals
				.len()
		};
		tracked(&host).xpect_eq(1);

		host.app.world_mut().entity_mut(host.host).despawn();

		tracked(&host).xpect_eq(0);
	}

	/// A small SVG exercising every feature the deck figures use — an internal
	/// `<style>` with class selectors, a `userSpaceOnUse` gradient, a filled
	/// polygon, a stroked path, a circle, and `<text>` — so the rasteriser is
	/// proven against the real surface area. `viewBox` is 100×60.
	#[cfg(all(feature = "tui", not(target_arch = "wasm32")))]
	const SAMPLE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 60">
  <style>.peak{fill:#72de5e}.line{fill:none;stroke:#8dfb77;stroke-width:3;stroke-linejoin:round}</style>
  <linearGradient id="g" x1="0" y1="0" x2="0" y2="60" gradientUnits="userSpaceOnUse">
    <stop offset="0" stop-color="#5fc24e"/><stop offset="1" stop-color="#0c3d10"/>
  </linearGradient>
  <rect width="100" height="60" fill="url(#g)" opacity="0.3"/>
  <polygon class="peak" points="10,55 30,15 50,55"/>
  <path class="line" d="M5,50 40,40 60,20 95,12"/>
  <circle cx="60" cy="20" r="3" fill="#8dfb77"/>
  <text x="50" y="58" font-family="sans-serif" font-size="8" fill="#e2e3dc" text-anchor="middle">Godot</text>
</svg>"##;

	/// An `<img src=*.svg>` is rasterised to a valid PNG at 2× the `viewBox`,
	/// covering the gradient/style/text surface the deck figures rely on.
	#[cfg(all(feature = "tui", not(target_arch = "wasm32")))]
	#[beet_core::test]
	fn rasterizes_svg_to_png() {
		// detected as svg, and not mistaken for a raster
		is_svg(SAMPLE_SVG.as_bytes()).xpect_true();
		is_svg(&png_bytes(8, 6)).xpect_false();
		// rasterised to a valid PNG at 2x the 100x60 viewBox
		to_png_bytes(SAMPLE_SVG.as_bytes().to_vec())
			.and_then(encode_png)
			.unwrap()
			.1
			.xpect_eq(UVec2::new(200, 120));
	}

	/// Dev aid (no assertions): with `BEET_SVG_DUMP_OUT` set, rasterise the file
	/// at `BEET_SVG_DUMP_IN` (or the built-in sample) to that PNG path, for
	/// eyeballing the terminal raster. Inert in a normal run.
	#[cfg(all(feature = "tui", not(target_arch = "wasm32")))]
	#[beet_core::test]
	fn dump_svg_raster() {
		let Ok(out) = env_ext::var("BEET_SVG_DUMP_OUT") else {
			return;
		};
		let svg = match env_ext::var("BEET_SVG_DUMP_IN") {
			Ok(path) => fs_ext::read(path.as_str()).unwrap(),
			Err(_) => SAMPLE_SVG.as_bytes().to_vec(),
		};
		fs_ext::write(out.as_str(), to_png_bytes(svg).unwrap()).unwrap();
	}

	/// SSH detection: a flattened `TERM` plus a non-zero pixel window (eg ghostty
	/// over SSH) enables graphics, while a plain terminal reporting no pixels keeps
	/// the alt marker. The term name alone still enables it without pixels.
	#[cfg(feature = "tui")]
	#[beet_core::test]
	fn pixel_window_enables_graphics() {
		KittyGraphicsSupport::from_pty(
			"xterm-256color",
			UVec2::new(1666, 2170),
		)
		.enabled
		.xpect_true();
		KittyGraphicsSupport::from_pty("xterm-256color", UVec2::ZERO)
			.enabled
			.xpect_false();
		KittyGraphicsSupport::from_pty("xterm-ghostty", UVec2::ZERO)
			.enabled
			.xpect_true();
	}

	/// An unsupported terminal keeps the `[image]: alt` marker fallback.
	#[cfg(feature = "tui")]
	#[beet_core::test]
	fn unsupported_terminal_keeps_marker() {
		let mut host = TestHost::sized(UVec2::new(40, 8));
		host.app
			.world_mut()
			.entity_mut(host.host)
			.insert(KittyGraphicsSupport { enabled: false });
		host.spawn_content(rsx! {
			<div><img src="missing.png" alt="fallback"/></div>
		});
		host.step();
		host.frame_plain().xpect_contains("[image]: fallback");
		String::from_utf8_lossy(&host.frame_ansi())
			.into_owned()
			.xnot()
			.xpect_contains("\u{1b}_G");
	}

	// ── item 9: the folk-technology blog post image ────────────────────────────
	// The real asset `<img src="/assets/blog/kiama-sea-shanty-club.jpg">`
	// references. Site-owned, so it lives under `site/assets/`; absent on a fresh
	// checkout until `just site-shared pull`, so the asset tests skip when missing.

	/// The site-rooted src of the folk-technology post image.
	#[cfg(all(feature = "tui", feature = "net", not(target_arch = "wasm32")))]
	const SHANTY_SRC: &str = "/assets/blog/kiama-sea-shanty-club.jpg";

	/// The real folk-technology JPEG, or `None` on a checkout without `assets/`.
	#[cfg(all(feature = "tui", feature = "net", not(target_arch = "wasm32")))]
	fn shanty_jpeg() -> Option<Vec<u8>> {
		fs_ext::read(
			AbsPathBuf::new_workspace_rel(
				"site/assets/blog/kiama-sea-shanty-club.jpg",
			)
			.unwrap(),
		)
		.ok()
	}

	/// The exact JPEG decode + PNG re-encode + dimension parse the renderer runs,
	/// on the real asset: it round-trips to a 1280x960 PNG.
	#[cfg(all(feature = "tui", feature = "net", not(target_arch = "wasm32")))]
	#[beet_core::test]
	fn shanty_jpeg_reencodes_to_png() {
		let Some(jpeg) = shanty_jpeg() else {
			return; // no local assets/ (fresh checkout); covered by `decodes_jpeg_image`
		};
		to_png_bytes(jpeg)
			.and_then(encode_png)
			.unwrap()
			.1
			.xpect_eq(UVec2::new(1280, 960));
	}

	/// THE item-9 regression guard: a site-rooted `/assets/…` `<img>` with no
	/// canonical server up (the pure-local `--server=tui` case) loopback-fetches,
	/// fails with the no-port error, and renders BOTH the `[image]: alt` marker and
	/// the styled error rather than a silent blank — and marks the element
	/// unavailable so the fetch is not retried.
	#[cfg(all(feature = "tui", feature = "net", not(target_arch = "wasm32")))]
	#[beet_core::test]
	async fn site_rooted_img_without_server_shows_alt_and_error() {
		// wide enough that the no-port message does not wrap mid-phrase.
		let mut host = TestHost::sized(UVec2::new(80, 8));
		// the fetch is queued async, so the host needs the async runtime.
		host.app.init_plugin::<AsyncPlugin>();
		// the host is its own surface, so the img resolves its graphics support.
		host.app.world_mut().entity_mut(host.host).insert((
			KittyGraphicsSupport { enabled: true },
			RenderSurface(host.host),
		));
		host.spawn_content(rsx! {
			<div><img src=SHANTY_SRC alt="shanty"/></div>
		});
		// the loopback fetch is async and fails (no canonical server bound): settle
		// until the element is marked unavailable, then the error fallback spawns.
		app_ext::update_until(&mut host.app, |world| {
			world
				.query_filtered::<(), With<KittyImageUnavailable>>()
				.iter(world)
				.next()
				.is_some()
		})
		.await
		.xpect_true();
		host.step();
		// both the alt marker (the `[image]: alt` gutter) and the no-port error from
		// the failed loopback fetch render — not a silent blank.
		host.frame_plain()
			.as_str()
			.xpect_contains("[image]: shanty")
			.xpect_contains(
				"local port not assigned, is the server running yet?",
			);
	}
}
