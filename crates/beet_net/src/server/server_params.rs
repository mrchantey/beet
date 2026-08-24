//! The bind knobs a booting server reads off its start request.
use crate::prelude::*;
use beet_core::prelude::*;
use core::net::IpAddr;

/// The bind knobs a booting server overlays onto its own declared config, read
/// from the start request.
///
/// The request alone, never the environment: env already fed each server's
/// [`Default`] through [`BootstrapConfig::get`], and a markup-declared field
/// out-ranks env, so consulting it again here would invert that precedence. A
/// start request is not a process launch, so this is a plain params type rather
/// than a second [`BootstrapConfig`]; `server_params_match_bootstrap_knobs` pins
/// its names to the flags a deploy renders, which is the drift the process config
/// exists to prevent.
#[derive(Debug, Default, Clone, PartialEq, Reflect)]
#[reflect(Default)]
pub struct ServerParams {
	/// The address to bind, overriding the declared host.
	pub host: Option<String>,
	/// The http listener port, overriding the declared port.
	pub port: Option<u16>,
	/// The ssh listener port, overriding the declared port.
	pub ssh_port: Option<u16>,
	/// The route a freshly-opened tui/ssh surface navigates to, overriding the
	/// request path.
	pub path: Option<String>,
}

impl ServerParams {
	/// The bind knobs `request` carries.
	pub fn from_request(request: &Request) -> Result<Self> {
		Self::from_parts(request.request_parts())
	}

	/// The bind knobs `parts` carry, for a facet that kept the start request's
	/// parts rather than the request itself.
	pub fn from_parts(parts: &RequestParts) -> Result<Self> {
		parts.params().parse_reflect()
	}

	/// The `--host` override as IPv4 octets, the form the server components hold.
	/// A malformed address errors; an IPv6 one warns and yields `None`, per
	/// [`BootstrapConfig::ipv4_octets`].
	pub fn host_octets(&self) -> Result<Option<[u8; 4]>> {
		self.host
			.as_deref()
			.map(|host| {
				host.parse::<IpAddr>()
					.map_err(|err| bevyhow!("invalid --host `{host}`: {err}"))
			})
			.transpose()?
			.and_then(BootstrapConfig::ipv4_octets)
			.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// The invariant that lets a start read plain params instead of a second
	/// [`BootstrapConfig`]: every flag the deploy renders for a bind knob is a
	/// flag [`ServerParams`] reads back. A renamed knob fails here rather than
	/// silently leaving a deployed server on the wrong port.
	#[beet_core::test]
	fn server_params_match_bootstrap_knobs() {
		let argv = BootstrapConfig {
			host: Some("0.0.0.0".parse().unwrap()),
			http_port: Some(9090),
			ssh_port: Some(2222),
			path: Some("docs/form".into()),
			..default()
		}
		.to_argv()
		.unwrap();
		let request = Request::from_cli_args(CliArgs::parse_tokens(argv));
		ServerParams::from_request(&request)
			.unwrap()
			.xpect_eq(ServerParams {
				host: Some("0.0.0.0".into()),
				port: Some(9090),
				ssh_port: Some(2222),
				path: Some("docs/form".into()),
			});
	}
}
