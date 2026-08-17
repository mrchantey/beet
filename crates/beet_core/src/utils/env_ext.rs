//! Cross-platform environment variable access.
//!
//! The wasm branches go through [`js_runtime`], which is a `std` surface, so each
//! switch names both; a `std`-less wasm build takes the same inert branch a bare
//! no_std target does.

use crate::prelude::*;
use thiserror::Error;

/// Error returned when an environment variable operation fails.
#[derive(Debug, Error)]
pub enum EnvError {
	/// The requested environment variable was not found.
	#[error("Environment variable not found: {0}")]
	NotFound(SmolStr),
	/// The platform has no process environment to mutate: a no_std target, or a
	/// js host that defines no env global (a browser, a Cloudflare Worker).
	/// Returned instead of silently succeeding, so a caller that depends on the
	/// mutation landing can say so.
	#[error("This platform has no process environment to mutate")]
	Unsupported,
}

/// Load environment variables from the nearest `.env` file, searching the current
/// directory and its ancestors. An existing variable always wins, and a missing
/// `.env` is not an error.
///
/// Errors with [`EnvError::Unsupported`] where there is no environment to load
/// into: a no_std target, or a js host without env globals (a browser takes its
/// configuration from the host page, a Worker from its bindings).
pub fn load_dotenv() -> Result<(), EnvError> {
	cfg_if! {
		if #[cfg(all(target_arch = "wasm32", feature = "std"))] {
			return js_runtime::load_dotenv()
				.then_some(())
				.ok_or(EnvError::Unsupported);
		} else if #[cfg(feature = "std")] {
			// a missing `.env` is the common case, not a failure.
			dotenv::dotenv().ok();
			return Ok(());
		} else {
			return Err(EnvError::Unsupported);
		}
	}
}

/// Get the command line arguments, excluding the program name
pub fn args() -> Vec<SmolStr> {
	cfg_if! {
		if #[cfg(all(target_arch = "wasm32", feature = "std"))] {
			// the wasm arg decision (deno argv, else browser location, else empty)
			// lives in `js_runtime`, so this stays a thin platform switch.
			return js_runtime::args();
		} else if #[cfg(feature = "std")] {
			return std::env::args().skip(1).map(SmolStr::from).collect();
		} else {
			return Vec::new();
		}
	}
}

/// Set an environment variable, erroring with [`EnvError::Unsupported`] where the
/// platform has no environment to mutate.
///
/// # Safety
/// Modifies global process state. Calling concurrently from multiple
/// threads or while other threads read environment variables is undefined behavior.
pub unsafe fn set_var(key: &str, value: &str) -> Result<(), EnvError> {
	cfg_if! {
		if #[cfg(all(target_arch = "wasm32", feature = "std"))] {
			// presence-checked + safe, so the absent-global case is an error
			// rather than a trap.
			return js_runtime::set_env(key, value)
				.then_some(())
				.ok_or(EnvError::Unsupported);
		} else if #[cfg(feature = "std")] {
			unsafe { std::env::set_var(key, value); }
			return Ok(());
		} else {
			let _ = (key, value);
			return Err(EnvError::Unsupported);
		}
	}
}

/// Remove an environment variable, erroring with [`EnvError::Unsupported`] where
/// the platform has no environment to mutate.
///
/// # Safety
/// Modifies global process state. Calling concurrently from multiple
/// threads or while other threads read environment variables is undefined behavior.
pub unsafe fn remove_var(key: &str) -> Result<(), EnvError> {
	cfg_if! {
		if #[cfg(all(target_arch = "wasm32", feature = "std"))] {
			// presence-checked + safe, so the absent-global case is an error
			// rather than a trap.
			return js_runtime::remove_env(key)
				.then_some(())
				.ok_or(EnvError::Unsupported);
		} else if #[cfg(feature = "std")] {
			unsafe { std::env::remove_var(key); }
			return Ok(());
		} else {
			let _ = key;
			return Err(EnvError::Unsupported);
		}
	}
}

/// Try get the environment variable with the given key, returning
/// an error containing the key name if not found.
pub fn var(key: &str) -> Result<SmolStr, EnvError> {
	cfg_if! {
		if #[cfg(all(target_arch = "wasm32", feature = "std"))] {
			return js_runtime::env_var(key)
				.ok_or_else(|| EnvError::NotFound(key.into()));
		} else if #[cfg(feature = "std")] {
			return std::env::var(key)
				.map(SmolStr::from)
				.map_err(|_| EnvError::NotFound(key.into()));
		} else {
			// no_std: no process environment, so always "not found" and callers
			// fall back to their defaults.
			return Err(EnvError::NotFound(key.into()));
		}
	}
}

/// Whether a windowing display server is actually reachable for a native window
/// runner (winit) to connect to.
///
/// On Linux/BSD winit needs a Wayland or X11 server; with neither reachable (a
/// headless box, CI, a bare SSH session) building the event loop panics, so a beet
/// binary falls back to the headless schedule loop instead. macOS, Windows and wasm
/// always report a display present, since their window runner has no such precondition.
///
/// The env vars alone are unreliable: WSLg sets `WAYLAND_DISPLAY` but leaves its
/// socket outside `XDG_RUNTIME_DIR`, and winit prefers Wayland and panics rather than
/// falling back to X11, so this verifies the socket winit will pick actually exists.
pub fn has_display() -> bool {
	cfg_if! {
		if #[cfg(all(feature = "std", any(target_os = "linux", target_os = "freebsd", target_os = "dragonfly", target_os = "netbsd", target_os = "openbsd")))] {
			unix_display_reachable()
		} else {
			true
		}
	}
}

/// Whether winit can reach a Wayland or X11 display on a unix host. winit prefers
/// Wayland when `WAYLAND_DISPLAY` is set and panics building the event loop if its
/// socket is unreachable (no fallback to X11), so a set-but-missing socket counts as
/// no display; only with `WAYLAND_DISPLAY` unset does it fall through to X11.
#[cfg(all(feature = "std", any(target_os = "linux", target_os = "freebsd", target_os = "dragonfly", target_os = "netbsd", target_os = "openbsd")))]
fn unix_display_reachable() -> bool {
	use std::path::Path;
	use std::path::PathBuf;
	// wayland: an absolute `WAYLAND_DISPLAY` is a socket path verbatim, else it is
	// relative to `XDG_RUNTIME_DIR`.
	if let Some(wayland) =
		var("WAYLAND_DISPLAY").ok().filter(|value| !value.is_empty())
	{
		let socket = if wayland.starts_with('/') {
			PathBuf::from(wayland.as_str())
		} else {
			match var("XDG_RUNTIME_DIR") {
				Ok(dir) => Path::new(dir.as_str()).join(wayland.as_str()),
				Err(_) => return false,
			}
		};
		return socket.exists();
	}
	// x11: a local `:N`/`unix:N` display is the socket `/tmp/.X11-unix/XN`; a remote
	// `host:N` display is assumed reachable (no local socket to stat).
	if let Some(display) = var("DISPLAY").ok().filter(|value| !value.is_empty()) {
		let (host, rest) =
			display.rsplit_once(':').unwrap_or(("", display.as_str()));
		if !host.is_empty() && host != "unix" {
			return true;
		}
		let number = rest.split('.').next().unwrap_or(rest);
		return Path::new("/tmp/.X11-unix").join(format!("X{number}")).exists();
	}
	false
}

/// Get all environment variables.
pub fn vars() -> Vec<(SmolStr, SmolStr)> {
	cfg_if! {
		if #[cfg(all(target_arch = "wasm32", feature = "std"))] {
			// `env_all` already marshals `Object.entries(Deno.env.toObject())`
			// into native pairs.
			return js_runtime::env_all();
		} else if #[cfg(feature = "std")] {
			return std::env::vars()
				.map(|(key, value)| (SmolStr::from(key), SmolStr::from(value)))
				.collect();
		} else {
			return Vec::new();
		}
	}
}

/// Get all environment variables that match the given filter.
pub fn vars_filtered(filter: GlobFilter) -> Vec<(SmolStr, SmolStr)> {
	vars()
		.into_iter()
		.filter(|(key, _)| filter.passes(key))
		.collect()
}
