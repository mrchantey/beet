//! Backend selection for [`Bert`].
//!
//! Exactly one of `wgpu` / `ndarray` / `cuda` should be enabled. The workspace
//! defaults to `wgpu` since it gives a single code-path that runs both natively
//! and in the browser; `ndarray` is the CPU-only path, for a host with no usable
//! GPU; `cuda` needs a system CUDA install.
use beet_core::prelude::cfg_if;

cfg_if! {
	if #[cfg(feature = "cuda")] {
		/// The active burn backend. Selected at compile time via cargo
		/// features (`wgpu` / `cuda` / `ndarray`).
		pub type DefaultBackend = burn::backend::Cuda;
		/// Returns the default device for [`DefaultBackend`].
		pub fn default_device() -> DefaultDevice { DefaultDevice::default() }
		/// [`default_device`]; only the wasm wgpu path needs async setup.
		pub async fn default_device_async() -> DefaultDevice { default_device() }
	} else if #[cfg(feature = "ndarray")] {
		/// The active burn backend.
		pub type DefaultBackend = burn::backend::NdArray;
		/// Returns the default device for [`DefaultBackend`].
		pub fn default_device() -> DefaultDevice { DefaultDevice::default() }
		/// [`default_device`]; only the wasm wgpu path needs async setup.
		pub async fn default_device_async() -> DefaultDevice { default_device() }
	} else if #[cfg(feature = "wgpu")] {
		// wgpu — also the path used in wasm
		/// The active burn backend.
		pub type DefaultBackend = burn::backend::Wgpu;
		/// Returns the device for [`DefaultBackend`]: the WGPU device shared from
		/// Bevy by [`SharedBurnWgpuPlugin`](crate::prelude::SharedBurnWgpuPlugin)
		/// when present, so Burn and Bevy share one GPU; else Burn's own default
		/// (the headless path, where Burn initialises its own device).
		pub fn default_device() -> DefaultDevice {
			crate::prelude::shared_burn_wgpu_device().unwrap_or_default()
		}
		/// Like [`default_device`], first completing the runtime setup wasm
		/// requires: cubecl cannot lazily block on adapter selection there, so
		/// the device must be initialized through the async path before first
		/// use. Native setup is lazy and this is just [`default_device`]. A
		/// bevy-shared device is already initialized and is returned as is.
		pub async fn default_device_async() -> DefaultDevice {
			let device = default_device();
			#[cfg(target_arch = "wasm32")]
			if !matches!(device, burn::backend::wgpu::WgpuDevice::Existing(_)) {
				burn::backend::wgpu::init_setup_async::<
					burn::backend::wgpu::graphics::AutoGraphicsApi,
				>(&device, Default::default())
				.await;
			}
			device
		}
	} else {
		compile_error!(
			"beet_ml needs exactly one burn backend feature: `wgpu` (the default, \
GPU, and the only wasm path), `ndarray` (CPU-only) or `cuda` (needs a system CUDA \
install). There is no backend-free build — every model runs on a burn device."
		);
	}
}

use burn::tensor::backend::BackendTypes;

/// Device type associated with [`DefaultBackend`].
pub type DefaultDevice = <DefaultBackend as BackendTypes>::Device;
