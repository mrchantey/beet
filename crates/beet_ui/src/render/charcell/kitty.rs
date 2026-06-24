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
//! gives it a block box. The measure phase sizes that box from the pixel
//! dimensions, paint reserves its cells, and `place_kitty_images` transmits the
//! bytes once and (re)places the picture whenever its on-screen rect changes —
//! scroll, reflow, or resize.
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

impl KittyImage {
	/// The cell footprint within `max_cols` columns: a nominal 10px column and
	/// the ~2:1 cell aspect, preserving the raster's aspect ratio. The terminal
	/// scales the image to exactly this rect (`c=`/`r=`).
	pub fn cell_size(&self, max_cols: u32) -> UVec2 {
		const CELL_PX_WIDTH: u32 = 10;
		let cols = self.px.x.div_ceil(CELL_PX_WIDTH).clamp(1, max_cols.max(1));
		UVec2::new(cols, self.rows_for(cols))
	}

	/// The cell footprint honoring explicit box dimensions: a missing axis
	/// derives from the raster's aspect, like a CSS replaced element with
	/// `width`/`height: auto`; with neither, [`cell_size`](Self::cell_size).
	pub fn cell_size_constrained(
		&self,
		width: Option<u32>,
		height: Option<u32>,
		max_cols: u32,
	) -> UVec2 {
		match (width, height) {
			(Some(cols), Some(rows)) => UVec2::new(cols.max(1), rows.max(1)),
			(Some(cols), None) => {
				let cols = cols.max(1);
				UVec2::new(cols, self.rows_for(cols))
			}
			(None, Some(rows)) => {
				// invert the 2:1 cell aspect: cols = rows * 2 * (px_w / px_h)
				let rows = rows.max(1);
				let cols = (rows * 2 * self.px.x)
					.div_ceil(self.px.y.max(1))
					.clamp(1, max_cols.max(1));
				UVec2::new(cols, rows)
			}
			(None, None) => self.cell_size(max_cols),
		}
	}

	/// Aspect-preserving rows for a `cols`-wide box, cells being ~2:1.
	fn rows_for(&self, cols: u32) -> u32 {
		(cols * self.px.y).div_ceil(self.px.x.max(1) * 2).max(1)
	}
}

/// The element state the attach system sets on a raster-backed `<img>`, giving
/// it the terminal-gated block box (see [`default_element_rules`]).
pub fn graphics_state() -> ElementState {
	ElementState::Custom("graphics".into())
}

/// Pixel dimensions from a PNG header (`IHDR` width/height), or `None` when
/// the bytes are not a PNG.
pub fn png_dimensions(bytes: &[u8]) -> Option<UVec2> {
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

/// Whether the host terminal renders the kitty graphics protocol, detected
/// from the environment. Overwrite the resource to force either way.
#[cfg(feature = "tui")]
#[derive(Debug, Clone, Resource)]
pub struct KittyGraphicsSupport {
	pub enabled: bool,
}

#[cfg(feature = "tui")]
impl Default for KittyGraphicsSupport {
	fn default() -> Self {
		let term = env_ext::var("TERM").unwrap_or_default();
		let enabled = term.contains("kitty")
			|| term.contains("ghostty")
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
pub fn attach_kitty_images(
	support: Res<KittyGraphicsSupport>,
	mut placements: ResMut<KittyPlacements>,
	elements: ElementQuery,
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
	if !support.enabled {
		return;
	}
	for view in elements.iter() {
		if view.tag() != "img" || !unvisited.contains(view.entity) {
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
pub fn render_image_errors(
	unavailable: Query<(Entity, &KittyImageUnavailable), Without<KittyErrorShown>>,
	mut commands: Commands,
) {
	for (entity, image) in unavailable.iter() {
		let error = image.error.clone();
		commands.entity(entity).insert(KittyErrorShown).with_children(
			|parent| {
				parent.spawn(rsx! { <Error>{error}</Error> });
			},
		);
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
	let loaded = fetch_image_bytes(&src).await.and_then(|bytes| {
		to_png_bytes(bytes).and_then(encode_png).ok_or_else(|| {
			bevyhow!("response is not a decodable image")
		})
	});
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

#[cfg(feature = "tui")]
#[derive(Debug, Clone, Copy, PartialEq)]
struct PlacedImage {
	id: u32,
	pos: UVec2,
	cells: UVec2,
}

#[cfg(feature = "tui")]
impl KittyPlacements {
	/// The next unused kitty image id.
	pub fn alloc_id(&mut self) -> u32 {
		self.next_id += 1;
		self.next_id
	}
}

/// ECS system: transmit and (re)place each visible [`KittyImage`] after the
/// cell renderer has drawn, diffing against [`KittyPlacements`] so escapes are
/// only emitted when an image appears, moves, resizes, or disappears.
#[cfg(feature = "tui")]
pub(crate) fn place_kitty_images(
	support: Res<KittyGraphicsSupport>,
	mut placements: ResMut<KittyPlacements>,
	mut terminals: Query<(Entity, &mut Terminal, &DoubleBuffer)>,
	charcell: CharcellQuery,
	tree: CharcellTree,
	images: Query<&KittyImage>,
) -> Result {
	if !support.enabled {
		return Ok(());
	}
	for (root, mut terminal, buffer) in terminals.iter_mut() {
		let viewport = buffer.size();
		let state = placements.terminals.entry(root).or_default();
		let writer = terminal.writer_mut();

		// a resize reallocated the screen: drop every placement and start over
		// (the cell renderer erased the screen; image data survives on the
		// terminal so only the cheap placements re-emit).
		if state.viewport != viewport {
			if !state.placed.is_empty() {
				write_delete_all(writer)?;
			}
			state.placed.clear();
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

/// The fully visible images under `root` and the screen rect each should
/// occupy, through the same scroll translation and clip the paint applied.
/// A partially clipped image is omitted (hidden) — the protocol places whole
/// rects, and a torn image is worse than none.
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
		// fully visible only: inside both the overflow clip and the screen
		let visible =
			cx.clip.intersect(rect) == rect && screen.intersect(rect) == rect;
		if !visible {
			continue;
		}
		desired.insert(entity, PlacedImage {
			id: image.id,
			pos: UVec2::new(rect.min.x as u32, rect.min.y as u32),
			cells: UVec2::new(rect.width() as u32, rect.height() as u32),
		});
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
#[cfg(feature = "tui")]
fn write_place(w: &mut (impl Write + ?Sized), placed: &PlacedImage) -> Result {
	escape::cursor_goto(&mut &mut *w, placed.pos)?;
	write!(
		w,
		"\x1b_Ga=p,i={},c={},r={},q=2,C=1\x1b\\",
		placed.id, placed.cells.x, placed.cells.y
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

	/// The cell box preserves aspect through the ~2:1 cell shape and clamps to
	/// the available columns.
	#[beet_core::test]
	fn cell_size_preserves_aspect() {
		let image = KittyImage {
			id: 1,
			data: String::new(),
			px: UVec2::new(200, 100),
		};
		// 200px / 10 = 20 cols; rows = 20 * (100/200) / 2 = 5
		image.cell_size(80).xpect_eq(UVec2::new(20, 5));
		// clamped to 10 cols, rows follow the aspect
		image.cell_size(10).xpect_eq(UVec2::new(10, 3));
	}

	// the live-terminal cases drive the `TestHost`/`KittyGraphicsSupport`
	// machinery, both `tui`-gated; default-feature builds skip them.
	#[cfg(feature = "tui")]
	use crate::render::charcell::test_host::TestHost;

	/// A host with graphics forced on and an `<img>` backed by a [`KittyImage`]
	/// of the given pixel dimensions, attached directly (no fetch) so the
	/// placement/transmission paths can be exercised without a server.
	#[cfg(feature = "tui")]
	fn image_host(width: u32, height: u32) -> TestHost {
		let mut host = TestHost::sized(UVec2::new(40, 14));
		host.app
			.insert_resource(KittyGraphicsSupport { enabled: true });
		host.spawn_content(rsx! {
			<div><img src="x.png" alt="a test image"/></div>
		});
		// attach the raster directly before the first step: the fetch path is
		// `net`-gated and needs a server, so seed the `KittyImage` so the attach
		// system skips the img (placement is independent of how the bytes arrived).
		let (data, px) =
			encode_png(png_bytes(width, height)).expect("valid png");
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
	/// placements and re-places from scratch.
	#[cfg(feature = "tui")]
	#[beet_core::test]
	fn removal_and_resize_replace_placements() {
		let mut host = image_host(100, 40);
		host.frame_ansi();
		// resize: every placement is dropped and re-emitted (payload retained)
		host.resize(UVec2::new(50, 16));
		host.step();
		let resized = String::from_utf8_lossy(&host.frame_ansi()).into_owned();
		resized
			.as_str()
			.xpect_contains("a=d,d=a,q=2")
			.xpect_contains("a=p,i=1");
		resized.xnot().xpect_contains("a=t");

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
		let Ok(out) = std::env::var("BEET_SVG_DUMP_OUT") else {
			return;
		};
		let svg = match std::env::var("BEET_SVG_DUMP_IN") {
			Ok(path) => fs_ext::read(path).unwrap(),
			Err(_) => SAMPLE_SVG.as_bytes().to_vec(),
		};
		fs_ext::write(out, to_png_bytes(svg).unwrap()).unwrap();
	}

	/// An unsupported terminal keeps the `[image]: alt` marker fallback.
	#[cfg(feature = "tui")]
	#[beet_core::test]
	fn unsupported_terminal_keeps_marker() {
		let mut host = TestHost::sized(UVec2::new(40, 8));
		host.app
			.insert_resource(KittyGraphicsSupport { enabled: false });
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
	// The real committed asset `<img src="/assets/blog/kiama-sea-shanty-club.jpg">`
	// references. Outside the crate, under the workspace `assets/`; absent on a
	// fresh checkout until `just pull-assets`, so the asset tests skip when missing.

	/// The site-rooted src of the folk-technology post image.
	#[cfg(all(feature = "tui", feature = "net"))]
	const SHANTY_SRC: &str = "/assets/blog/kiama-sea-shanty-club.jpg";

	/// The real folk-technology JPEG, or `None` on a checkout without `assets/`.
	#[cfg(all(feature = "tui", feature = "net", not(target_arch = "wasm32")))]
	fn shanty_jpeg() -> Option<Vec<u8>> {
		fs_ext::read(
			AbsPathBuf::new_workspace_rel("assets/blog/kiama-sea-shanty-club.jpg")
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
		host.app
			.init_plugin::<AsyncPlugin>()
			.insert_resource(KittyGraphicsSupport { enabled: true });
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
			.xpect_contains("local port not assigned, is the server running yet?");
	}
}
