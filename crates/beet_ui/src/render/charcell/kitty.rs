//! Raster images in the terminal via the kitty graphics protocol.
//!
//! Supported terminals (kitty, ghostty, WezTerm) draw real images over the
//! cell grid using APC escapes. `attach_kitty_images` loads each `<img>`'s
//! `src` — an absolute or site-rooted URL is fetched over HTTP (a site-rooted
//! `/assets/…` against [`RenderServerOrigin`], our own server, which maps it to
//! its blob store), a bare path reads the local filesystem; PNG bytes transmit
//! directly, any other format decodes and re-encodes to PNG — then attaches a
//! [`KittyImage`] and the `graphics` element
//! state so the terminal-gated user-agent rule gives it a block box. The
//! measure phase sizes that box from the pixel dimensions, paint reserves its
//! cells, and `place_kitty_images` transmits the bytes once and (re)places the
//! picture whenever its on-screen rect changes — scroll, reflow, or resize.
//! Unsupported terminals keep the `[image]: alt` marker fallback.
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
		let cols = self
			.px
			.x
			.div_ceil(CELL_PX_WIDTH)
			.clamp(1, max_cols.max(1));
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
			(Some(cols), Some(rows)) => {
				UVec2::new(cols.max(1), rows.max(1))
			}
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

/// The base [`Url`] a site-rooted image `src` (`/assets/…`) is fetched from, so
/// the terminal renderer requests the image over HTTP from our own server exactly
/// like the browser does — the server maps the path to its blob store (fs, S3,
/// …), with no filesystem dependency on the render host (Lambda/Fargate). Set by
/// [`set_render_server_origin`] from the running `HttpServer`'s loopback address;
/// absent (no HTTP server listening), a `/`-rooted `src` is unresolvable and the
/// `[image]: alt` marker presents.
#[cfg(all(feature = "tui", feature = "net"))]
#[derive(Debug, Clone, Resource)]
pub struct RenderServerOrigin(pub beet_net::prelude::Url);

/// Observer: point [`RenderServerOrigin`] at our own `HttpServer`'s loopback
/// address when one lands, so a site-rooted image `src` is fetched over HTTP and
/// mapped to the blob store (no filesystem on the render host). A `port: 0`
/// (OS-assigned, pre-bind) is skipped; the fixed port is used as-is.
#[cfg(all(feature = "tui", feature = "net"))]
pub fn set_render_server_origin(
	ev: On<Insert, beet_net::prelude::HttpServer>,
	servers: Query<&beet_net::prelude::HttpServer>,
	mut commands: Commands,
) {
	use beet_net::prelude::Url;
	if let Ok(port) =
		servers.get(ev.entity).map(|server| server.socket_addr().port())
		&& port != 0
	{
		commands.insert_resource(RenderServerOrigin(Url::parse(format!(
			"http://127.0.0.1:{port}"
		))));
	}
}

/// Marks an `<img>` whose `src` could not back a raster (missing file, not a
/// PNG, a failed or unsupported fetch), so the attach system tries it exactly
/// once and the `[image]: alt` marker fallback presents it.
#[cfg(feature = "tui")]
#[derive(Debug, Clone, Copy, Component)]
pub struct KittyImageUnavailable;

/// Marks an `<img>` whose remote `src` is being fetched, so exactly one fetch
/// is in flight. The alt marker presents until the bytes arrive.
#[cfg(feature = "tui")]
#[derive(Debug, Clone, Copy, Component)]
pub struct KittyImageLoading;

/// ECS system: back each new `<img>` with a [`KittyImage`] when the terminal
/// supports graphics. A local `src` loads synchronously; an `http(s)` one
/// fetches in the background (`net` feature) and attaches on arrival.
#[cfg(feature = "tui")]
pub fn attach_kitty_images(
	support: Res<KittyGraphicsSupport>,
	mut placements: ResMut<KittyPlacements>,
	#[cfg(feature = "net")] origin: Option<Res<RenderServerOrigin>>,
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
		// an absolute or site-rooted URL is fetched over HTTP from our own server
		// (which maps it to the blob store, no filesystem); needs the `net` client.
		#[cfg(feature = "net")]
		if let Some(url) =
			image_request_url(&src, origin.as_ref().map(|origin| &origin.0))
		{
			let id = placements.alloc_id();
			commands.entity(view.entity).insert(KittyImageLoading);
			commands
				.entity(view.entity)
				.queue_async(move |entity| fetch_remote(entity, url, id));
			continue;
		}
		// a bare/relative path is read from the local filesystem (test fixtures,
		// offline renders); a site-rooted path with no server origin is unresolvable.
		match load_local_image(&src) {
			Some((data, px)) => {
				let image = KittyImage {
					id: placements.alloc_id(),
					data,
					px,
				};
				commands
					.entity(view.entity)
					.queue(move |entity: EntityWorldMut| {
						attach_image(entity, image)
					});
			}
			None => {
				commands.entity(view.entity).insert(KittyImageUnavailable);
			}
		}
	}
}

/// Insert the raster and the `graphics` element state driving its block box,
/// merging into any states the element already carries (eg hover).
#[cfg(feature = "tui")]
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

/// The [`Url`](beet_net::prelude::Url) an image `src` is fetched from over HTTP,
/// or `None` for a local-file path read directly. An absolute `http(s)://` URL is
/// used as-is; a site-rooted `/assets/…` is resolved against the server `origin`
/// (our own server, which maps it to the blob store) like the browser does; a
/// bare/relative path is a local file (`None`).
#[cfg(all(feature = "tui", feature = "net"))]
fn image_request_url(
	src: &str,
	origin: Option<&beet_net::prelude::Url>,
) -> Option<beet_net::prelude::Url> {
	use beet_net::prelude::Url;
	if src.starts_with("http://") || src.starts_with("https://") {
		Some(Url::parse(src))
	} else if src.starts_with('/') {
		origin.map(|origin| Url::parse(format!("{origin}{src}")))
	} else {
		None
	}
}

/// Read and encode a local-file image `src` (a bare/relative path), or `None`
/// when missing or undecodable. Non-PNG bytes (eg a `.jpg`) are decoded and
/// re-encoded to PNG, the one format the kitty `t=d,f=100` path transmits.
#[cfg(feature = "tui")]
fn load_local_image(src: &str) -> Option<(String, UVec2)> {
	if src.is_empty() {
		return None;
	}
	encode_png(to_png_bytes(fs_ext::read(src).ok()?)?)
}

/// PNG bytes for an image: PNG input passes through; any other format the
/// `image` decoder understands (eg JPEG) is decoded to RGBA and re-encoded to
/// PNG. `None` when the bytes are not a decodable image.
#[cfg(feature = "tui")]
fn to_png_bytes(bytes: Vec<u8>) -> Option<Vec<u8>> {
	if png_dimensions(&bytes).is_some() {
		return Some(bytes);
	}
	let image = image::load_from_memory(&bytes).ok()?;
	let mut png = std::io::Cursor::new(Vec::new());
	image
		.write_to(&mut png, image::ImageFormat::Png)
		.ok()
		.map(|_| png.into_inner())
}

/// Background fetch for a remote `src`: attach the raster on arrival, or mark
/// the element unavailable so the alt marker stays.
#[cfg(all(feature = "tui", feature = "net"))]
async fn fetch_remote(
	entity: AsyncEntity,
	url: beet_net::prelude::Url,
	id: u32,
) -> Result {
	use beet_net::prelude::*;
	// no `Accept` constraint: the server returns the stored file (jpg/png/…) and
	// `to_png_bytes` decodes it; pinning `Accept: png` would reject a jpg asset.
	let loaded = async {
		Request::get(url)
			.send()
			.await
			.ok()?
			.into_result()
			.await
			.ok()?
			.bytes_vec()
			.await
			.ok()
	}
	.await
	.and_then(to_png_bytes)
	.and_then(encode_png);
	entity
		.with(move |mut entity| {
			entity.remove::<KittyImageLoading>();
			match loaded {
				Some((data, px)) => {
					attach_image(entity, KittyImage { id, data, px });
				}
				None => {
					entity.insert(KittyImageUnavailable);
				}
			}
		})
		.await
}

/// Validate and base64-encode PNG bytes, with their parsed dimensions.
#[cfg(feature = "tui")]
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

		let desired = desired_placements(
			root, viewport, &charcell, &tree, &images,
		);

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
					.ok_or_else(|| bevyhow!("missing image {}", placed.id))?;
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
		let visible = cx.clip.intersect(rect) == rect
			&& screen.intersect(rect) == rect;
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
fn write_transmit(w: &mut (impl Write + ?Sized), id: u32, data: &str) -> Result {
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

	/// A host with graphics forced on and an `<img>` whose `src` is a real
	/// PNG file in a temp dir.
	// the temp file + `fs_ext` load is native-only (no filesystem on wasm).
	#[cfg(all(feature = "tui", not(target_arch = "wasm32")))]
	fn image_host(width: u32, height: u32) -> (TestHost, TempDir) {
		let mut host = TestHost::sized(UVec2::new(40, 14));
		host.app
			.insert_resource(KittyGraphicsSupport { enabled: true });
		let dir = TempDir::new().unwrap();
		let path = dir.path().join("test.png");
		fs_ext::write(&path, png_bytes(width, height)).unwrap();
		let src = path.to_string_lossy().to_string();
		host.spawn_content(rsx! {
			<div><img src=src alt="a test image"/></div>
		});
		host.step();
		(host, dir)
	}

	/// A supported terminal transmits the PNG once and places it at its
	/// laid-out cell rect; the alt-text fallback is not painted.
	#[cfg(all(feature = "tui", not(target_arch = "wasm32")))]
	#[beet_core::test]
	fn transmits_and_places_image() {
		let (mut host, _dir) = image_host(100, 40);
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

	/// An image `src` resolves to an HTTP request for a URL or a site-rooted path
	/// (against the server origin), and a bare path stays a local-file read.
	#[cfg(all(feature = "tui", feature = "net"))]
	#[beet_core::test]
	fn image_url_resolution() {
		use beet_net::prelude::Url;
		let origin = Url::parse("http://127.0.0.1:8339");
		let url = |src| {
			image_request_url(src, Some(&origin)).map(|url| url.to_string())
		};
		// a site-rooted path fetches from our own server (mapped to the blob store)
		url("/assets/x.jpg")
			.xpect_eq(Some("http://127.0.0.1:8339/assets/x.jpg".to_string()));
		// an absolute URL is used as-is
		url("https://cdn/x.png")
			.xpect_eq(Some("https://cdn/x.png".to_string()));
		// no origin: a site-rooted path is unresolvable (no HTTP, no local read)
		image_request_url("/assets/x.jpg", None).is_none().xpect_true();
		// a bare path is a local file, not an HTTP request
		image_request_url("logo.png", Some(&origin)).is_none().xpect_true();
	}

	/// A non-PNG image (a JPEG) is decoded and re-encoded to PNG so the kitty
	/// `f=100` transmit handles it, with its dimensions preserved; a local-file
	/// `src` reads from disk and decodes.
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
		to_png_bytes(jpeg.clone())
			.and_then(encode_png)
			.unwrap()
			.1
			.xpect_eq(UVec2::new(8, 6));
		// a local-file jpeg src loads and decodes
		let dir = TempDir::new().unwrap();
		fs_ext::write(dir.path().join("x.jpg"), &jpeg).unwrap();
		load_local_image(&dir.path().join("x.jpg").to_string_lossy())
			.unwrap()
			.1
			.xpect_eq(UVec2::new(8, 6));
	}

	/// Removing the image deletes its placement; a resize deletes all visible
	/// placements and re-places from scratch.
	#[cfg(all(feature = "tui", not(target_arch = "wasm32")))]
	#[beet_core::test]
	fn removal_and_resize_replace_placements() {
		let (mut host, _dir) = image_host(100, 40);
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
}
