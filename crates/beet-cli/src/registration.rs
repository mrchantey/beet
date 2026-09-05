//! This binary's own compiled surface, as an entry's `<CrateCheck/>` reads it.
use beet::prelude::*;

/// Every feature `beet-cli` can be compiled with, each recorded if enabled, so
/// an entry's `<CrateCheck/>` and the `--features` flag verify against the
/// running binary. Spawned by every entry driver (the native binary, the wasm
/// binary, the Worker) before the entry loads.
///
/// The PRIMARY registration: an unprefixed requirement resolves here. A
/// downstream binary spawns its own instead, naming its own features, which is
/// why this is beet-cli's and not the launch core's.
pub fn cli() -> CrateRegistration {
	crate_registration!({
		features: [
			"aws_sdk",
			"cloudflare",
			"extra",
			"geoip",
			"infra",
			"lambda",
			"ml",
			"net",
			"pdf",
			"qrcode",
			"secure",
			"sockets",
			"ssh",
			"thread",
			"tui",
			"web",
			"web_examples",
			"web_head",
			"winit",
		]
	})
	.with_skip_prefix()
}
