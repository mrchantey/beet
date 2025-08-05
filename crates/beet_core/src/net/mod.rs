mod cross_fetch;
#[cfg(not(target_arch = "wasm32"))]
mod impl_reqwest;
#[cfg(target_arch = "wasm32")]
mod impl_web_sys;
