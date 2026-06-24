//! Backend selection for [`Bert`].
//!
//! Exactly one of `wgpu` / `ndarray` / `cuda` should be enabled — the
//! workspace defaults to `wgpu` since it gives a single code-path that
//! runs both natively and in the browser.
use beet_core::prelude::cfg_if;

cfg_if! {
	if #[cfg(feature = "cuda")] {
		/// The active burn backend. Selected at compile time via cargo
		/// features (`wgpu` / `cuda` / `ndarray`).
		pub type DefaultBackend = burn::backend::Cuda;
		/// Returns the default device for [`DefaultBackend`].
		pub fn default_device() -> DefaultDevice { DefaultDevice::default() }
	} else if #[cfg(feature = "ndarray")] {
		/// The active burn backend.
		pub type DefaultBackend = burn::backend::NdArray;
		/// Returns the default device for [`DefaultBackend`].
		pub fn default_device() -> DefaultDevice { DefaultDevice::default() }
	} else {
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
	}
}

use burn::tensor::backend::BackendTypes;

/// Device type associated with [`DefaultBackend`].
pub type DefaultDevice = <DefaultBackend as BackendTypes>::Device;
