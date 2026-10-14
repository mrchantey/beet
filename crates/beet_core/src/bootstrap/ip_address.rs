//! An IP address as a reflectable leaf.
use crate::prelude::*;
use core::fmt;
use core::net::AddrParseError;
use core::net::IpAddr;
use core::net::Ipv4Addr;
use core::str::FromStr;

/// An [`IpAddr`] a reflected type may hold, ie the `--host` launch knob.
///
/// `bevy_reflect` has no [`IpAddr`] impl, so this opaque leaf carries one,
/// authored as its usual text (`0.0.0.0`) through its [`LiteralParser`] entry.
///
/// ```
/// # use beet_core::prelude::*;
/// let host: IpAddress = "0.0.0.0".parse().unwrap();
/// host.to_string().xpect_eq("0.0.0.0");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deref, Reflect)]
#[reflect(opaque, Debug, PartialEq, Hash)]
#[cfg_attr(
	feature = "serde",
	derive(serde::Serialize, serde::Deserialize),
	serde(transparent)
)]
pub struct IpAddress(pub IpAddr);

impl From<IpAddr> for IpAddress {
	fn from(addr: IpAddr) -> Self { Self(addr) }
}

impl From<Ipv4Addr> for IpAddress {
	fn from(addr: Ipv4Addr) -> Self { Self(addr.into()) }
}

impl FromStr for IpAddress {
	type Err = AddrParseError;
	fn from_str(value: &str) -> Result<Self, Self::Err> {
		value.trim().parse::<IpAddr>().map(Self)
	}
}

impl fmt::Display for IpAddress {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { self.0.fmt(f) }
}
