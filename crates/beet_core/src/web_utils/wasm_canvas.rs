//! Pixels in the tab: canvas surface wiring for a winit-less browser render
//! boot.
//!
//! A wasm render runner boots windowless with no winit backend, so a
//! scene-spawned [`Window`] entity has nothing to hand it a surface.
//! `bevy_render` needs only a [`RawHandleWrapper`] on the `Window` entity to
//! create one (its extract queries `(&Window, &RawHandleWrapper)`, winit never
//! involved), and wgpu's WebGPU backend resolves a `RawWindowHandle::Web` to
//! the canvas matching the `[data-raw-handle="<id>"]` DOM selector. This
//! module supplies that missing half: on every new `Window` it resolves the
//! page's `#beet-canvas` (a page-supplied mount point) or creates one appended
//! to `body`, stamps the `data-raw-handle` attribute, and inserts a
//! `RawHandleWrapper` built from an id-only handle shim, after which the
//! render stack surfaces and presents to the canvas like any native window.
//!
//! Pixels ride only the WebGPU boot: a GPU-less fallback compiles no backend
//! that could create a surface, so it stays surfaceless (the plugin is inert
//! there and no canvas is touched). WebGL2 stays out of scope: its adapter
//! exists only off a canvas-backed surface at init time, a different boot
//! order to this windowless one.

use crate::prelude::*;
use bevy::window::RawHandleWrapper;
use bevy::window::Window;
use bevy::window::WindowWrapper;
use raw_window_handle::DisplayHandle;
use raw_window_handle::HandleError;
use raw_window_handle::HasDisplayHandle;
use raw_window_handle::HasWindowHandle;
use raw_window_handle::RawWindowHandle;
use raw_window_handle::WebWindowHandle;
use raw_window_handle::WindowHandle;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use web_sys::HtmlCanvasElement;

/// The canvas id a browser render boot draws into: a page supplies its own
/// mount point as `<canvas id="beet-canvas">`, else one is created and
/// appended to `body`.
pub const BEET_CANVAS_ID: &str = "beet-canvas";

/// Adds the canvas surface systems for a winit-less browser render runner
/// (the facade's `wasm_render_runner` registers this). Inert on a GPU-less
/// boot, which has no backend to surface and stays surfaceless.
pub struct WasmCanvasPlugin;

impl Plugin for WasmCanvasPlugin {
	fn build(&self, app: &mut App) {
		if !js_runtime::has_webgpu() {
			return;
		}
		app.add_systems(PreUpdate, (attach_canvas, sync_canvas_size).chain())
			.add_observer(remove_canvas);
	}
}

/// The canvas claimed for one `Window`: the `data-raw-handle` id wgpu resolves
/// the element by, and whether beet created the element (a created canvas is
/// removed with the window; a page-supplied `#beet-canvas` is left in place).
#[derive(Component)]
struct CanvasSurface {
	raw_handle: u32,
	created: bool,
}

impl CanvasSurface {
	/// The claimed canvas element, resolved by its stamped `data-raw-handle`
	/// (the element itself is a JS value, so the component stores only the id).
	fn canvas(&self) -> Option<HtmlCanvasElement> {
		document_ext::query_selector(&format!(
			"[data-raw-handle=\"{}\"]",
			self.raw_handle
		))
	}
}

/// An id-only window/display handle for a `data-raw-handle`-stamped canvas:
/// the wasm twin of a winit window in [`RawHandleWrapper`]'s eyes, carrying no
/// pointer at all (wgpu re-resolves the canvas from the id's DOM selector).
struct CanvasHandle(u32);

impl HasWindowHandle for CanvasHandle {
	fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
		let raw = RawWindowHandle::Web(WebWindowHandle::new(self.0));
		// SAFETY: an id-only handle with no pointer is valid for any lifetime.
		unsafe { WindowHandle::borrow_raw(raw) }.xok()
	}
}

impl HasDisplayHandle for CanvasHandle {
	fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
		DisplayHandle::web().xok()
	}
}

/// Give every new [`Window`] a canvas-backed surface: resolve or create the
/// canvas, stamp the `data-raw-handle` attribute, adopt the browser's pixel
/// ratio, and insert the [`RawHandleWrapper`] `bevy_render` surfaces from.
fn attach_canvas(
	mut commands: Commands,
	mut windows: Query<(Entity, &mut Window), Added<Window>>,
) -> Result {
	for (entity, mut window) in windows.iter_mut() {
		// ids from 1: winit's convention, and 0 reads as unset.
		static NEXT_RAW_HANDLE: AtomicU32 = AtomicU32::new(1);
		let raw_handle = NEXT_RAW_HANDLE.fetch_add(1, Ordering::Relaxed);
		let (canvas, created) = resolve_canvas()?;
		canvas
			.set_attribute("data-raw-handle", &raw_handle.to_string())
			.map_err(|_| bevyhow!("failed to stamp canvas handle"))?;
		// physical px are real here: surface and canvas backing store render at
		// the display's pixel ratio while the logical resolution stays authored.
		let ratio = document_ext::window().device_pixel_ratio() as f32;
		window.resolution.set_scale_factor(ratio);
		let surface = CanvasSurface {
			raw_handle,
			created,
		};
		let wrapper = RawHandleWrapper::new(&WindowWrapper::new(
			CanvasHandle(raw_handle),
		))?;
		commands.entity(entity).insert((surface, wrapper));
	}
	Ok(())
}

/// The unclaimed `#beet-canvas` (page-supplied), else a created one appended
/// to `body`. A second `Window` skips an already-claimed (stamped) canvas and
/// creates its own.
fn resolve_canvas() -> Result<(HtmlCanvasElement, bool)> {
	if let Some(canvas) = document_ext::query_selector::<HtmlCanvasElement>(
		&format!("#{BEET_CANVAS_ID}:not([data-raw-handle])"),
	) {
		return (canvas, false).xok();
	}
	let canvas = document_ext::create_canvas();
	if document_ext::query_selector::<HtmlCanvasElement>(&format!(
		"#{BEET_CANVAS_ID}"
	))
	.is_none()
	{
		canvas.set_id(BEET_CANVAS_ID);
	}
	document_ext::body()
		.append_child(&canvas)
		.map_err(|_| bevyhow!("failed to append canvas to body"))?;
	(canvas, true).xok()
}

/// Keep the canvas's css size at the `Window`'s logical resolution. Only the
/// display size needs syncing: wgpu sizes the backing store (the width/height
/// attributes) itself on every surface configure, driven by the physical
/// resolution `bevy_render` extracts from the same `Window`.
fn sync_canvas_size(
	windows: Query<(&Window, &CanvasSurface), Changed<Window>>,
) -> Result {
	for (window, surface) in windows.iter() {
		let Some(canvas) = surface.canvas() else {
			continue;
		};
		canvas
			.set_attribute(
				"style",
				&format!(
					"width:{}px;height:{}px",
					window.resolution.width(),
					window.resolution.height()
				),
			)
			.map_err(|_| bevyhow!("failed to style canvas"))?;
	}
	Ok(())
}

/// Remove a beet-created canvas with its window (surface teardown is
/// `bevy_render`'s, reacting to the removed [`RawHandleWrapper`]); a
/// page-supplied canvas only loses its claim stamp.
fn remove_canvas(
	ev: On<Remove, CanvasSurface>,
	surfaces: Query<&CanvasSurface>,
) {
	let Ok(surface) = surfaces.get(ev.entity) else {
		return;
	};
	let Some(canvas) = surface.canvas() else {
		return;
	};
	if surface.created {
		canvas.remove();
	} else {
		let _ = canvas.remove_attribute("data-raw-handle");
	}
}
