//! The `--wasm-host` selection grammar.

use crate::prelude::*;
use core::fmt;
use core::str::FromStr;

/// The host `beet run-wasm` executes a wasm module in, parsed once from
/// `--wasm-host` / `BEET_WASM_HOST`.
///
/// ## Example
///
/// ```
/// # use beet_core::prelude::*;
/// let host: WasmHost = "browser".parse().unwrap();
/// host.xpect_eq(WasmHost::Browser);
/// ```
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Reflect)]
#[reflect(Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WasmHost {
	/// The bundled deno runner: a js runtime with fs access and no dom.
	#[default]
	Deno,
	/// A headless browser driven through the webdriver: the host for
	/// `#[beet_core::test(browser)]` dom suites.
	Browser,
}

impl FromStr for WasmHost {
	type Err = String;
	fn from_str(value: &str) -> Result<Self, Self::Err> {
		match value.to_lowercase().as_str() {
			"deno" => Ok(WasmHost::Deno),
			"browser" => Ok(WasmHost::Browser),
			other => Err(format!(
				"invalid wasm host `{other}`, expected `deno` or `browser`"
			)),
		}
	}
}

impl fmt::Display for WasmHost {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			WasmHost::Deno => write!(f, "deno"),
			WasmHost::Browser => write!(f, "browser"),
		}
	}
}
