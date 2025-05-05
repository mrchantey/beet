use anyhow::Result;
use colorize::AnsiColor;
use http::Uri;
use std::fmt::Display;
use sweet_utils::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct Address {
	pub host: [u8; 4],
	pub port: u16,
	pub port_tls: u16,
	pub secure: bool,
}

impl Address {
	pub fn to_uri(&self) -> Result<Uri> {
		let uri = self.to_string().parse()?;
		Ok(uri)
	}

	pub fn host_from_str(host: &str) -> anyhow::Result<[u8; 4]> {
		let mut parts = host.split('.');
		let v1 = parts.next().or_err()?.parse::<u8>()?;
		let v2 = parts.next().or_err()?.parse::<u8>()?;
		let v3 = parts.next().or_err()?.parse::<u8>()?;
		let v4 = parts.next().or_err()?.parse::<u8>()?;
		Ok([v1, v2, v3, v4])
	}
	pub fn is_all(&self) -> bool {
		self.host[0] == 0
			&& self.host[1] == 0
			&& self.host[2] == 0
			&& self.host[3] == 0
	}
	pub fn to_socket_addr(&self) -> std::net::SocketAddr {
		self.socket_addr(self.port)
	}
	pub fn to_socket_addr_tls(&self) -> std::net::SocketAddr {
		self.socket_addr(self.port_tls)
	}
	fn socket_addr(&self, port: u16) -> std::net::SocketAddr {
		std::net::SocketAddr::new(
			std::net::IpAddr::V4(std::net::Ipv4Addr::new(
				self.host[0],
				self.host[1],
				self.host[2],
				self.host[3],
			)),
			port,
		)
	}
	pub fn to_string_pretty(&self) -> String {
		self.to_string().b_cyan().underlined().bold()
	}
}

impl Default for Address {
	fn default() -> Self {
		Self {
			host: [0, 0, 0, 0],
			port: 7777,
			port_tls: 7778,
			secure: false,
		}
	}
}
impl Display for Address {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let host = if self.is_all() {
			&[127, 0, 0, 1]
		} else {
			&self.host
		};
		let (prefix, port) = if self.secure {
			("https://", self.port_tls)
		} else {
			("http://", self.port)
		};

		let str = format!(
			"{}{}.{}.{}.{}:{}",
			prefix, host[0], host[1], host[2], host[3], port
		);
		write!(f, "{str}")
	}
}
