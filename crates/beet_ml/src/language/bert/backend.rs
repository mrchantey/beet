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
	} else if #[cfg(feature = "ndarray")] {
		/// The active burn backend.
		pub type DefaultBackend = burn::backend::NdArray;
	} else {
		// wgpu — also the path used in wasm
		/// The active burn backend.
		pub type DefaultBackend = burn::backend::Wgpu;
	}
}

use burn::tensor::backend::BackendTypes;

/// Device type associated with [`DefaultBackend`].
pub type DefaultDevice = <DefaultBackend as BackendTypes>::Device;

/// Returns the default device for [`DefaultBackend`].
pub fn default_device() -> DefaultDevice { DefaultDevice::default() }
