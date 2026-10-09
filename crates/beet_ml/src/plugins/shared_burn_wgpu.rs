//! Share Bevy's WGPU device with Burn, so ML inference and rendering run on a
//! single GPU context instead of two competing `wgpu` instances (the "shared GPU"
//! problem a Burn wgpu backend hits when it spins up alongside `bevy_render`).
//!
//! Recipe adapted from Jan Hohenheim's writeup:
//! <https://github.com/janhohenheim/share_wgpu_between_bevy_and_burn/blob/main/readme.md>

/// Shares Bevy's WGPU device, queue, adapter and instance with Burn's wgpu backend,
/// so ML inference and rendering draw on one GPU context. Add it after Bevy's render
/// plugins; in [`finish`](Plugin::finish) it captures the
/// [`RenderApp`](bevy::render::RenderApp) wgpu handles (cheap `Arc` clones), and the
/// first [`default_device`](crate::prelude::default_device) call registers them with
/// Burn. Lazy on purpose: registering compiles Burn's kernels on the device, so a
/// binary with ml compiled in but not used pays (and risks) nothing.
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
static SHARED_DEVICE: std::sync::OnceLock<burn::tensor::Device> =
	std::sync::OnceLock::new();

/// The WGPU device shared from Bevy by [`SharedBurnWgpuPlugin`], initialising
/// Burn on it on first call, or `None` when no Bevy device has been shared (a
/// headless Burn app, where Burn makes its own).
#[cfg(feature = "wgpu")]
pub(crate) fn shared_burn_wgpu_device() -> Option<burn::tensor::Device> {
	if let Some(device) = SHARED_DEVICE.get() {
		return Some(device.clone());
	}
	beet_core::prelude::cfg_if! {
		if #[cfg(feature = "bevy_default")] {
			share::lazy_init_shared_device()
		} else {
			None
		}
	}
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
	use burn::tensor::Device;
	use burn::tensor::wgpu::WgpuSetup;
	use std::sync::Mutex;

	/// Bevy's wgpu handles, captured at plugin finish and consumed by the first
	/// [`lazy_init_shared_device`] call.
	static SHARED_SETUP: Mutex<Option<WgpuSetup>> = Mutex::new(None);

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
			// capture bevy's instance/adapter/device/queue: each render resource
			// derefs to its wgpu handle, itself a cheap (Arc-backed) clone. Burn
			// initialises on them lazily, on the first `default_device`.
			let setup = WgpuSetup {
				instance: (**world.resource::<RenderInstance>()).clone(),
				adapter: (**world.resource::<RenderAdapter>()).clone(),
				device: world.resource::<RenderDevice>().wgpu_device().clone(),
				queue: (**world.resource::<RenderQueue>()).clone(),
				backend: world.resource::<RenderAdapterInfo>().backend,
			};
			*SHARED_SETUP.lock().unwrap() = Some(setup);
		}
	}

	/// Initialise Burn on the captured Bevy handles, caching the device. The
	/// lock is held across the registration so a concurrent caller waits rather
	/// than falling back to a second wgpu instance. A setup Burn rejects (ie a
	/// device lacking the compute limits it needs) leaves Burn on its own device.
	pub(super) fn lazy_init_shared_device() -> Option<Device> {
		let mut setup_slot = SHARED_SETUP.lock().unwrap();
		if let Some(device) = SHARED_DEVICE.get() {
			return Some(device.clone());
		}
		let setup = setup_slot.take()?;
		match Device::wgpu_options().setup(setup).init() {
			Ok(device) => {
				info!(
					"SharedBurnWgpuPlugin: Burn now shares Bevy's WGPU device"
				);
				SHARED_DEVICE.get_or_init(|| device).clone().xsome()
			}
			Err(err) => {
				warn!(
					"SharedBurnWgpuPlugin: Burn rejected Bevy's WGPU device, \
					falling back to its own: {err}"
				);
				None
			}
		}
	}
}

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

#[cfg(all(
	test,
	feature = "bevy_default",
	feature = "wgpu",
	not(target_arch = "wasm32")
))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use burn::tensor::Tensor;

	/// Captures the device Bevy's renderer created and runs a Burn tensor op
	/// on it.
	#[beet_core::test]
	async fn shares_bevy_device() {
		// bevy's renderer needs an adapter too, so a host granting Burn none skips
		if !has_device().await {
			warn!("no gpu adapter granted, skipping shares_bevy_device");
			return;
		}
		// the render stack, windowless: natively the renderer is created while
		// the plugins build, so `finish` hands its handles straight to the plugin
		let mut app = App::new();
		app.add_plugins((
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: None,
					exit_condition: bevy::window::ExitCondition::DontExit,
					..default()
				})
				.disable::<bevy::winit::WinitPlugin>()
				.disable::<bevy::log::LogPlugin>()
				.disable::<bevy::audio::AudioPlugin>()
				.disable::<bevy::gilrs::GilrsPlugin>(),
			SharedBurnWgpuPlugin,
		));
		app.finish();
		app.cleanup();

		let device = shared_burn_wgpu_device().unwrap();
		default_device().await.unwrap().xpect_eq(device.clone());
		Tensor::<1>::from_data([1f32, 2., 3.].as_slice(), &device)
			.sum()
			.into_scalar::<f32>()
			.xpect_eq(6.);
	}
}
