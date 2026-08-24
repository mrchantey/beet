// The `Script`/`ScriptAction` types marshal their `Input`/`Output` through
// serde, and JSON is the wire currency of every backend, so the `scripting`
// feature pulls both (`serde` + `json`) and one gate serves them.
#[cfg(feature = "scripting")]
mod script;
#[cfg(feature = "scripting")]
mod script_action;
#[cfg(feature = "scripting")]
pub use script::*;
#[cfg(feature = "scripting")]
pub use script_action::*;

// The wire format, compiled with the host-realm backends that speak it and no
// wider: the embedded engine calls the engine directly and needs none of it.
// It is also not part of beet's surface — a consumer names `Script`, never a
// `ScriptRequest` — so the re-export is crate-internal.
#[cfg(all(feature = "scripting", feature = "std", not(feature = "quickjs")))]
pub(crate) mod protocol;
#[cfg(all(
	feature = "scripting",
	feature = "std",
	not(feature = "quickjs")
))]
pub(crate) use protocol::*;

// The embedded engine: a separate axis from `scripting`, and a target-agnostic
// one. `quickjs` compiles it in everywhere, wasm included.
#[cfg(feature = "quickjs")]
mod quickjs_backend;
#[cfg(feature = "quickjs")]
pub(crate) use quickjs_backend::run_quickjs;
#[cfg(feature = "quickjs")]
pub(crate) use quickjs_backend::run_quickjs_console;

// The host-realm fallbacks, compiled only when the embedded engine is absent.
// Each borrows its isolation from the surrounding runtime, so which ones exist
// is a property of the target: a sandboxed child process on native, a child
// isolate on wasm (picked between at runtime, since the wasm host is not known
// until the module boots). `std` throughout — a bare-metal target has no host
// realm to borrow from, which leaves the embedded engine as its only option.
beet_core::cfg_if! {
	if #[cfg(all(
		feature = "scripting",
		feature = "std",
		not(feature = "quickjs"),
	))] {
		beet_core::cfg_if! {
			if #[cfg(target_arch = "wasm32")] {
				mod cloudflare_backend;
				mod iframe_backend;
				mod deno_worker_backend;
				pub(crate) use cloudflare_backend::run_cloudflare;
				pub(crate) use iframe_backend::run_iframe;
				pub(crate) use deno_worker_backend::run_deno_worker;
			} else {
				mod deno_cli_backend;
				pub(crate) use deno_cli_backend::run_deno_cli;
			}
		}
	}
}
