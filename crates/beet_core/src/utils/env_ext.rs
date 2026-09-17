//! Cross-platform environment variable access.
//!
//! The wasm branches go through [`js_runtime`], which is a `std` surface, so each
//! switch names both; a `std`-less wasm build takes the same inert branch a bare
//! no_std target does. A js host with no env globals (a browser tab, a Worker)
//! reads and writes an in-memory map, so config flows through here on every
//! target that has `std`.

use crate::prelude::*;
use thiserror::Error;

/// Error returned when an environment variable operation fails.
#[derive(Debug, Error)]
pub enum EnvError {
	/// The requested environment variable was not found.
	#[error("Environment variable not found: {0}")]
	NotFound(SmolStr),
	/// The platform has no process environment to mutate, ie a no_std target.
	/// Returned instead of silently succeeding, so a caller that depends on the
	/// mutation landing can say so.
	#[error("This platform has no process environment to mutate")]
	Unsupported,
}

/// Load environment variables from the nearest `.env` file, searching the current
/// directory and its ancestors, and with the `secrets` feature from the
/// `.env.age` vault beside it. An existing variable always wins, then `.env`,
/// then `.env.age`; a missing file is not an error.
///
/// One implementation on every platform: the files are found and read through
/// [`fs_ext`] (whose wasm arm is the js host's fs globals), parsed by
/// [`parse_dotenv`] and written through [`set_var`], so a deno runner and a native
/// process resolve the same file the same way.
///
/// `.env.age` decrypts with the identity `AgeIdentityFile::discover` finds. A
/// present vault and no identity (a cloud box, a contributor without one) is a
/// single warning naming both, never an error: every binary calls this before
/// it has a logger, so the warning goes to stderr directly.
///
/// Errors with [`EnvError::Unsupported`] where there is no environment to load
/// into, ie a no_std target; a js host without a filesystem finds no `.env`.
pub fn load_dotenv() -> Result<(), EnvError> {
	cfg_if! {
		if #[cfg(feature = "std")] {
			// a missing `.env` is the common case, not a failure.
			return match find_dotenv_dir() {
				Some(dir) => load_dotenv_from(&dir),
				None => Ok(()),
			};
		} else {
			return Err(EnvError::Unsupported);
		}
	}
}

/// [`load_dotenv`] for a caller that knows the directory: `dir`'s `.env` and
/// `.env.age`, no ancestor walk, the same precedence.
#[cfg(feature = "std")]
pub fn load_dotenv_from(dir: &std::path::Path) -> Result<(), EnvError> {
	#[cfg(feature = "secrets")]
	let pairs = dotenv_pairs(dir, AgeIdentityFile::discover);
	#[cfg(not(feature = "secrets"))]
	let pairs = env_pairs(dir);
	set_missing(pairs)
}

/// The plaintext file name.
#[cfg(feature = "std")]
const DOTENV: &str = ".env";
/// The vault beside it, see [`load_dotenv`].
#[cfg(feature = "std")]
const DOTENV_AGE: &str = ".env.age";

/// The directory `.env` and `.env.age` are loaded from: the nearest ancestor
/// of the current directory holding a `.env`, else the nearest holding a
/// `.env.age`, so a vault beside a plaintext file is read with it and a lone
/// vault is found the same way. `None` when no ancestor has either (or the
/// host has no filesystem).
#[cfg(feature = "std")]
fn find_dotenv_dir() -> Option<std::path::PathBuf> {
	let cwd = fs_ext::current_dir().ok()?;
	[DOTENV, DOTENV_AGE].into_iter().find_map(|name| {
		cwd.ancestors()
			.find(|dir| fs_ext::exists(dir.join(name)).unwrap_or(false))
			.map(std::path::Path::to_path_buf)
	})
}

/// Set every pair whose key is not already set, in order, so the first
/// source of a key wins.
#[cfg(feature = "std")]
fn set_missing(
	pairs: impl IntoIterator<Item = (SmolStr, SmolStr)>,
) -> Result<(), EnvError> {
	pairs
		.into_iter()
		.filter(|(key, _)| var(key).is_err())
		// SAFETY: process-wide mutation, so this is a startup call made
		// before any other thread reads the environment.
		.try_for_each(|(key, value)| unsafe { set_var(&key, &value) })
}

/// The pairs of `dir`'s `.env`, empty when there is none.
#[cfg(feature = "std")]
fn env_pairs(dir: &std::path::Path) -> Vec<(SmolStr, SmolStr)> {
	fs_ext::read_to_string(dir.join(DOTENV))
		.map(|contents| parse_dotenv(&contents))
		.unwrap_or_default()
}

/// The pairs of `dir`'s `.env` followed by its `.env.age`, the vault
/// decrypted with what `identities` resolves. Every vault failure (no
/// identity, an identity the vault was not encrypted to, a corrupt file) is
/// one warning to stderr rather than an error, since a contributor without
/// the identity must still build.
#[cfg(feature = "secrets")]
fn dotenv_pairs(
	dir: &std::path::Path,
	identities: impl FnOnce() -> Result<Option<AgeIdentityFile>>,
) -> Vec<(SmolStr, SmolStr)> {
	let mut pairs = env_pairs(dir);
	let path = dir.join(DOTENV_AGE);
	let Ok(ciphertext) = fs_ext::read(&path) else {
		return pairs;
	};
	let identities = match identities() {
		Ok(Some(identities)) => identities,
		Ok(None) => {
			cross_log_error!(
				"warning: `{}` is present but no age identity was found at `{}` \
				(or `{}`), so it was not loaded: `beet secrets/keygen` makes one, \
				`secrets/restore-identity` restores a backup",
				path.display(),
				AgeIdentityFile::default_path()
					.map(|path| path.display().to_string())
					.unwrap_or_default(),
				AgeIdentityFile::ENV_VAR
			);
			return pairs;
		}
		Err(err) => {
			cross_log_error!(
				"warning: `{}` was not loaded, the age identity did not \
				resolve: {err}",
				path.display()
			);
			return pairs;
		}
	};
	match VaultDocument::decrypt(VaultFormat::Env, &identities, &ciphertext) {
		Ok(VaultDocument::Env(doc)) => pairs.extend(doc.pairs()),
		Ok(VaultDocument::Tree(_)) => {}
		Err(err) => cross_log_error!(
			"warning: `{}` was not loaded: {err}",
			path.display()
		),
	}
	pairs
}

/// Parse `.env` contents into `(key, value)` pairs through the
/// [`EnvDocument`] grammar: blank lines and `#` comments are skipped, a leading
/// `export ` is dropped, a value wrapped in matching single or double quotes is
/// unwrapped, and a line without a `=` is skipped.
///
/// The single dotenv grammar in beet, so a caller loading a `.env` from somewhere
/// other than the filesystem (a blob store entry, a host page) parses it
/// identically to [`load_dotenv`], and a `.env.age` vault reads the same way.
pub fn parse_dotenv(contents: &str) -> Vec<(SmolStr, SmolStr)> {
	EnvDocument::parse(contents).pairs()
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
			// the host's env global, else the in-memory fallback
			js_runtime::set_env(key, value);
			return Ok(());
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
			// the host's env global, and the in-memory fallback either way
			js_runtime::remove_env(key);
			return Ok(());
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
#[cfg(all(
	feature = "std",
	any(
		target_os = "linux",
		target_os = "freebsd",
		target_os = "dragonfly",
		target_os = "netbsd",
		target_os = "openbsd"
	)
))]
fn unix_display_reachable() -> bool {
	use std::path::Path;
	use std::path::PathBuf;
	// wayland: an absolute `WAYLAND_DISPLAY` is a socket path verbatim, else it is
	// relative to `XDG_RUNTIME_DIR`.
	if let Some(wayland) = var("WAYLAND_DISPLAY")
		.ok()
		.filter(|value| !value.is_empty())
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
	if let Some(display) = var("DISPLAY").ok().filter(|value| !value.is_empty())
	{
		let (host, rest) =
			display.rsplit_once(':').unwrap_or(("", display.as_str()));
		if !host.is_empty() && host != "unix" {
			return true;
		}
		let number = rest.split('.').next().unwrap_or(rest);
		return Path::new("/tmp/.X11-unix")
			.join(format!("X{number}"))
			.exists();
	}
	false
}

/// Get all environment variables.
pub fn vars() -> Vec<(SmolStr, SmolStr)> {
	cfg_if! {
		if #[cfg(all(target_arch = "wasm32", feature = "std"))] {
			// `env_all` already marshals `Object.entries(Deno.env.toObject())`
			// (plus the in-memory fallback) into native pairs.
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

#[cfg(test)]
mod test {
	use crate::prelude::*;

	// comments, blanks, `export`, quoting and an `=` inside a value.
	#[crate::test]
	fn parses_dotenv() {
		env_ext::parse_dotenv(
			"# a comment\n\nFOO=bar\nexport BAZZ='boo'\nBOOM=\"a b\"\nURL=http://x?a=b\nnot a pair\n",
		)
		.xpect_eq(vec![
			(SmolStr::new("FOO"), SmolStr::new("bar")),
			(SmolStr::new("BAZZ"), SmolStr::new("boo")),
			(SmolStr::new("BOOM"), SmolStr::new("a b")),
			(SmolStr::new("URL"), SmolStr::new("http://x?a=b")),
		]);
	}

	// every test host (native, and the deno runner over the js fs globals) runs
	// inside the workspace, so the ancestor walk reaches its `.env`.
	#[crate::test]
	fn loads_dotenv() { env_ext::load_dotenv().unwrap(); }

	/// `.env.age` loads after `.env`, so the plaintext wins a shared key and
	/// the process environment wins both. Native only: the deno host has no
	/// writable temp dir behind `fs_ext`.
	#[cfg(all(feature = "secrets", not(target_arch = "wasm32")))]
	#[crate::test]
	fn env_age_loads_after_env() {
		let identity = AgeIdentity::generate();
		let mut identities = AgeIdentityFile::default();
		identities.push(identity.clone());
		let dir = std::env::temp_dir()
			.join(format!("beet-dotenv-{}", Timestamp::now().millis()));
		fs_ext::create_dir_all(&dir).unwrap();
		fs_ext::write(
			dir.join(".env"),
			"BEET_TEST_DOTENV_SHARED=plain\nBEET_TEST_DOTENV_PLAIN=1\n",
		)
		.unwrap();
		let vault = VaultDocument::parse(
			VaultFormat::Env,
			"BEET_TEST_DOTENV_SHARED=vault\nBEET_TEST_DOTENV_VAULT=2\n",
		)
		.unwrap();
		fs_ext::write(
			dir.join(".env.age"),
			vault
				.encrypt(VaultFormat::Env, &[identity.to_recipient()])
				.unwrap(),
		)
		.unwrap();

		// the plaintext pairs come first, so the existing-wins rule keeps them
		let pairs = super::dotenv_pairs(&dir, || Ok(Some(identities)));
		pairs.len().xpect_eq(4);
		super::set_missing(pairs).unwrap();
		env_ext::var("BEET_TEST_DOTENV_SHARED")
			.unwrap()
			.xpect_eq("plain");
		env_ext::var("BEET_TEST_DOTENV_PLAIN")
			.unwrap()
			.xpect_eq("1");
		env_ext::var("BEET_TEST_DOTENV_VAULT")
			.unwrap()
			.xpect_eq("2");

		// no identity: the vault is skipped with a warning, `.env` still loads
		super::dotenv_pairs(&dir, || Ok(None)).len().xpect_eq(2);
		// the wrong identity: the same
		let mut other = AgeIdentityFile::default();
		other.push(AgeIdentity::generate());
		super::dotenv_pairs(&dir, || Ok(Some(other)))
			.len()
			.xpect_eq(2);
		// SAFETY: test-only, keys no other test reads
		unsafe {
			for key in [
				"BEET_TEST_DOTENV_SHARED",
				"BEET_TEST_DOTENV_PLAIN",
				"BEET_TEST_DOTENV_VAULT",
			] {
				env_ext::remove_var(key).unwrap();
			}
		}
		fs_ext::remove(&dir).unwrap();
	}
}
