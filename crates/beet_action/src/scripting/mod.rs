// The `Script`/`ScriptAction` types marshal their `Input`/`Output` through
// `Value`, so they need `serde`; running one needs a backend, so they are gated
// on `serde` + at least one usable backend (a build with neither has no
// `ScriptLanguage` variant, so the typed `Script` could not pick a default).
// `run_rhai` needs `rhai` + `serde`; `run_quickjs` additionally JSON-marshals
// across the engine boundary, so it needs `json` and is native-only.
#[cfg(all(feature = "quickjs", feature = "json", not(target_arch = "wasm32")))]
mod quickjs_runtime;
#[cfg(all(feature = "rhai", feature = "serde"))]
mod rhai_runtime;
#[cfg(all(
	feature = "serde",
	any(feature = "rhai", all(feature = "quickjs", not(target_arch = "wasm32")))
))]
mod script;
#[cfg(all(
	feature = "serde",
	any(feature = "rhai", all(feature = "quickjs", not(target_arch = "wasm32")))
))]
mod script_action;
#[cfg(all(feature = "quickjs", feature = "json", not(target_arch = "wasm32")))]
pub(crate) use quickjs_runtime::run_quickjs;
#[cfg(all(feature = "rhai", feature = "serde"))]
pub(crate) use rhai_runtime::run_rhai;
#[cfg(all(
	feature = "serde",
	any(feature = "rhai", all(feature = "quickjs", not(target_arch = "wasm32")))
))]
pub use script::*;
#[cfg(all(
	feature = "serde",
	any(feature = "rhai", all(feature = "quickjs", not(target_arch = "wasm32")))
))]
pub use script_action::*;
