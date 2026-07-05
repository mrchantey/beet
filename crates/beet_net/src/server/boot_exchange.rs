//! The no_std core of the boot path: the [`Boot`] exchange newtype and the
//! [`request_selects_server`] selection predicate.
//!
//! Both are needed by the unconditionally compiled [`HttpServer`](super::HttpServer)
//! observer (a `StartRunning<Boot>` listener that reads `--server` to decide
//! whether to boot), so they compile on no_std. The std-only load verbs and
//! `AppExit` writers that drive them — [`BootOnLoad`](super::BootOnLoad),
//! [`Boot::boot`](super::Boot::boot), the stdout streamer — live in
//! [`boot`](super::boot).
use crate::prelude::*;
use beet_core::prelude::*;

/// A [`Request`] wrapped as a *boot* exchange, kept distinct from a dispatch
/// exchange so the two action slots on a host never collide.
///
/// A server host holds an `Action<Boot, Response>` (the boot slot, a
/// `ContinueRun`) alongside an `Action<Request, Response>` (dispatch). Booting
/// calls the former with `Boot(request)`; dispatching calls the latter with the
/// [`Request`]. Because `Boot` and [`Request`] are different types the two slots
/// coexist with no holder newtype.
///
/// Needs no `Clone`: `StartRunning` clones via an `Arc` with no `In: Clone` bound,
/// and [`Request`] is itself not `Clone`.
#[derive(Debug, Deref, DerefMut)]
pub struct Boot(pub Request);

impl From<Request> for Boot {
	fn from(request: Request) -> Self { Self(request) }
}

/// Whether a server named `name` should boot for `request`, read from its
/// `--server` params. Reads every `server` value (repeated flags) and splits each
/// on commas (a glob list, eg `--server=cli,http`); the name must pass the
/// resulting [`GlobFilter`].
///
/// With no `--server` param the `BEET_SERVER` env is the fallback, so a deployed
/// binary launched with no args (a lambda bootstrap, a lightsail systemd unit)
/// selects its transport that way. Absent both, the server's own `default_boot`
/// decides: it defaults to `true`, so a bare `beet` brings up every declared
/// server, and an entry clears it on one (eg a secondary [`HttpServer`]) that
/// should boot only when `--server` names it.
pub fn request_selects_server(
	request: &Request,
	name: &str,
	default_boot: bool,
) -> bool {
	let mut globs = request
		.get_params("server")
		.into_iter()
		.flatten()
		.map(|value| value.to_string())
		.collect::<Vec<_>>();
	// absent an explicit `--server`, the `BEET_SERVER` env selects the servers.
	if globs.is_empty() {
		globs.extend(env_ext::var("BEET_SERVER").ok());
	}
	// absent both, fall back to this server's own default.
	if globs.is_empty() {
		return default_boot;
	}
	globs
		.iter()
		.flat_map(|value| value.split(','))
		.map(str::trim)
		.filter(|name| !name.is_empty())
		.fold(GlobFilter::default(), |filter, name| {
			filter.with_include(name)
		})
		.passes(name)
}
