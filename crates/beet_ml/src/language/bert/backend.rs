//! Device selection for [`Bert`].
//!
//! Burn dispatches on the [`Device`] rather than a backend type parameter, so
//! models and tensors carry no backend generic: the cargo feature decides which
//! backend [`default_device`] names. Exactly one of `wgpu` / `flex` / `cuda`
//! should be enabled. The workspace defaults to `wgpu` since it gives a single
//! code-path that runs both natively and in the browser; `flex` is Burn's
//! pure-Rust CPU path, for a host with no usable GPU; `cuda` needs a system CUDA
//! install.
use beet_core::prelude::*;
use burn::tensor::Device;

cfg_if! {
	if #[cfg(feature = "cuda")] {
		/// The device every model runs on, selected at compile time via cargo
		/// features (`wgpu` / `cuda` / `flex`).
		pub async fn default_device() -> Result<Device> {
			Device::cuda(burn::tensor::DeviceIndex::Default).xok()
		}
		/// Whether this host grants a device for [`default_device`]. cuda is
		/// selected explicitly, so a host without it is a build error rather than
		/// something to probe around.
		pub async fn has_device() -> bool { true }
	} else if #[cfg(feature = "flex")] {
		/// The device every model runs on, selected at compile time via cargo
		/// features (`wgpu` / `cuda` / `flex`).
		pub async fn default_device() -> Result<Device> { Device::flex().xok() }
		/// Whether this host grants a device for [`default_device`]; the CPU
		/// backend always has one.
		pub async fn has_device() -> bool { true }
	} else if #[cfg(feature = "wgpu")] {
		use burn::tensor::DeviceKind;
		use burn::tensor::wgpu::WgpuInitError;
		use std::sync::OnceLock;

		/// Burn's own wgpu device once initialized, so later calls skip the
		/// adapter acquisition [`default_device`] otherwise repeats.
		static OWN_DEVICE: OnceLock<Device> = OnceLock::new();

		/// The device every model runs on: the WGPU device shared from Bevy by
		/// [`SharedBurnWgpuPlugin`](crate::prelude::SharedBurnWgpuPlugin) when
		/// present, so Burn and Bevy share one GPU; else Burn's own, initialized
		/// on the default adapter (the headless path).
		///
		/// The initialization is async because wasm cannot block on adapter
		/// selection, so a device must be initialized through here before first
		/// use there. Errors when the host grants no adapter.
		pub async fn default_device() -> Result<Device> {
			if let Some(device) = crate::prelude::shared_burn_wgpu_device() {
				return device.xok();
			}
			if let Some(device) = OWN_DEVICE.get() {
				return device.clone().xok();
			}
			let device = match Device::wgpu_options().init_async().await {
				Ok(device) => device,
				// registered already, by a concurrent caller or a tensor op's lazy
				// init: the lazy handle names that same runtime
				Err(WgpuInitError::Registration { .. }) => {
					Device::wgpu(DeviceKind::DefaultDevice)
				}
				Err(err) => return Err(err.into()),
			};
			OWN_DEVICE.get_or_init(|| device).clone().xok()
		}
		/// Whether this host grants a device for [`default_device`], so anything
		/// that merely needs *a* GPU, rather than asserting one exists, can skip
		/// when the answer is no (GPU-less CI, a host whose driver is broken).
		/// Wasm asks the host through `js_runtime::probe_webgpu` before
		/// initializing.
		pub async fn has_device() -> bool {
			#[cfg(target_arch = "wasm32")]
			if crate::prelude::shared_burn_wgpu_device().is_none()
				&& !js_runtime::probe_webgpu().await
			{
				return false;
			}
			default_device().await.is_ok()
		}
	} else {
		compile_error!(
			"beet_ml needs exactly one burn backend feature: `wgpu` (the default, \
GPU, and the only wasm path), `flex` (CPU-only) or `cuda` (needs a system CUDA \
install). There is no backend-free build — every model runs on a burn device."
		);
	}
}
