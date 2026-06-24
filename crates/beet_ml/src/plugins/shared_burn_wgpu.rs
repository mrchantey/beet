//! Share Bevy's WGPU device with Burn, so ML inference and rendering run on a
//! single GPU context instead of two competing `wgpu` instances (the "shared GPU"
//! problem a Burn wgpu backend hits when it spins up alongside `bevy_render`).
//!
//! Recipe adapted from Jan Hohenheim's writeup:
//! <https://github.com/janhohenheim/share_wgpu_between_bevy_and_burn/blob/main/readme.md>

/// Shares Bevy's WGPU device, queue, adapter and instance with Burn's wgpu backend,
/// so ML inference and rendering draw on one GPU context. Add it after Bevy's render
/// plugins; in [`finish`](Plugin::finish) it reads the
/// [`RenderApp`](bevy::render::RenderApp) wgpu resources, registers them with Burn
/// via `init_device`, and stores the resulting [`BurnWgpuDevice`].
/// [`default_device`](crate::prelude::default_device) then hands that device to every
/// [`Bert`](crate::prelude::Bert) model, so loading a model reuses Bevy's GPU rather
/// than initialising a second wgpu instance.
///
/// Inert unless both `bevy_default` (Bevy's render stack) and `wgpu` (Burn's wgpu
/// backend) are enabled: a headless Burn app, or a non-wgpu backend, needs neither
/// and the plugin is a no-op there.
#[derive(Default)]
pub struct SharedBurnWgpuPlugin;

// the shared-device slot, read by `default_device` (`backend.rs`) on the wgpu
// backend. Lives under `wgpu` alone (not `bevy_default`) so a wgpu-but-headless
// build still compiles the getter — it just stays `None`.
#[cfg(feature = "wgpu")]
static SHARED_DEVICE: std::sync::OnceLock<burn::backend::wgpu::WgpuDevice> =
	std::sync::OnceLock::new();

/// The WGPU device [`SharedBurnWgpuPlugin`] shared from Bevy, or `None` when no
/// Bevy device has been shared (a headless Burn app, where Burn makes its own).
#[cfg(feature = "wgpu")]
pub fn shared_burn_wgpu_device() -> Option<burn::backend::wgpu::WgpuDevice> {
	SHARED_DEVICE.get().cloned()
}

// the real device-sharing impl: needs Bevy's render stack (`bevy_default`) AND
// Burn's wgpu backend (`wgpu`).
#[cfg(all(feature = "bevy_default", feature = "wgpu"))]
mod share {
	use super::SHARED_DEVICE;
	use super::SharedBurnWgpuPlugin;
	use beet_core::prelude::*;
	use bevy::render::RenderApp;
	use bevy::render::renderer::RenderAdapter;
	use bevy::render::renderer::RenderAdapterInfo;
	use bevy::render::renderer::RenderDevice;
	use bevy::render::renderer::RenderInstance;
	use bevy::render::renderer::RenderQueue;
	use bevy::render::renderer::WgpuWrapper;
	use burn::backend::wgpu::RuntimeOptions;
	use burn::backend::wgpu::WgpuDevice;
	use burn::backend::wgpu::WgpuSetup;
	use burn::backend::wgpu::init_device;
	use std::sync::Arc;

	/// Bevy's WGPU device, shared with Burn and stored so ml systems/assets can
	/// name it. The same handle [`default_device`](crate::prelude::default_device)
	/// returns, so a `Bert` model and the renderer share one GPU.
	#[derive(Debug, Clone, Resource, Deref)]
	pub struct BurnWgpuDevice(pub WgpuDevice);

	impl Plugin for SharedBurnWgpuPlugin {
		fn build(&self, _app: &mut App) {}
		// runs after `RenderPlugin::finish` has created the RenderApp + its wgpu
		// resources (plugin `finish` runs in add order, and this is added after
		// the Bevy default plugins), so the device/queue/adapter exist here.
		fn finish(&self, app: &mut App) {
			let Some(render_app) = app.get_sub_app(RenderApp) else {
				warn!(
					"SharedBurnWgpuPlugin: no RenderApp, skipping wgpu share"
				);
				return;
			};
			let world = render_app.world();
			// reuse bevy's instance/adapter/device/queue instead of making a second set.
			let setup = WgpuSetup {
				instance: clone_inner(&world.resource::<RenderInstance>().0),
				adapter: clone_inner(&world.resource::<RenderAdapter>().0),
				device: world.resource::<RenderDevice>().wgpu_device().clone(),
				queue: clone_inner(&world.resource::<RenderQueue>().0),
				backend: world.resource::<RenderAdapterInfo>().backend,
			};
			let device = init_device(setup, RuntimeOptions::default());
			let _ = SHARED_DEVICE.set(device.clone());
			app.insert_resource(BurnWgpuDevice(device));
			info!("SharedBurnWgpuPlugin: Burn now shares Bevy's WGPU device");
		}
	}

	// bevy wraps each wgpu handle in `Arc<WgpuWrapper<T>>`; the inner handle is
	// itself a cheap (Arc-backed) `Clone`, reached here by deref coercion.
	fn clone_inner<T: Clone>(wrapped: &Arc<WgpuWrapper<T>>) -> T {
		<T as Clone>::clone(wrapped)
	}
}
#[cfg(all(feature = "bevy_default", feature = "wgpu"))]
pub use share::BurnWgpuDevice;

// inert when sharing is not meaningful (a headless Burn app, or a non-wgpu
// backend): the plugin type still exists so an app can always add it unconditionally.
#[cfg(not(all(feature = "bevy_default", feature = "wgpu")))]
use beet_core::prelude::App;
#[cfg(not(all(feature = "bevy_default", feature = "wgpu")))]
use beet_core::prelude::Plugin;
#[cfg(not(all(feature = "bevy_default", feature = "wgpu")))]
impl Plugin for SharedBurnWgpuPlugin {
	fn build(&self, _app: &mut App) {}
}
