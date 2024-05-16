use lightyear::client::config::ClientConfig;
use lightyear::client::plugin::ClientPlugins;
use lightyear::prelude::*;
use std::net::Ipv4Addr;
use std::net::SocketAddr;




pub struct MinimalSettings {
	server_addr: Ipv4Addr,
	server_port: u16,
	netcode_protocol_id: u64,
	netcode_private_key: [u8; 32],
}

impl Default for MinimalSettings {
	fn default() -> Self {
		Self {
			server_addr: Ipv4Addr::new(0, 0, 0, 0),
			netcode_protocol_id: 0,
			netcode_private_key: default(),
		}
	}
}


pub fn minimal_client() { ClientPlugins::new(ClientConfig {}); }
