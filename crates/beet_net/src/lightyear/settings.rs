//! This module parses the settings.ron file and builds a lightyear configuration from it
#[cfg(not(target_family = "wasm"))]
use bevy::utils::Duration;
use lightyear::prelude::client::Authentication;
use lightyear::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use std::net::Ipv4Addr;
use std::net::SocketAddr;

#[derive(
	Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
pub enum ClientTransports {
	#[cfg(not(target_family = "wasm"))]
	Udp,
	#[cfg(feature = "lightyear/webtransport")]
	WebTransport { certificate_digest: String },
	#[cfg(feature = "lightyear/websocket")]
	WebSocket,
	#[cfg(not(target_family = "wasm"))]
	#[cfg(feature = "lightyear/steam")]
	Steam { app_id: u32 },
}

#[derive(
	Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
pub enum ServerTransports {
	Udp {
		local_port: u16,
	},
	#[cfg(feature = "lightyear/webtransport")]
	WebTransport {
		local_port: u16,
	},
	#[cfg(feature = "lightyear/websocket")]
	WebSocket {
		local_port: u16,
	},
	#[cfg(not(target_family = "wasm"))]
	#[cfg(feature = "lightyear/steam")]
	Steam {
		app_id: u32,
		server_ip: Ipv4Addr,
		game_port: u16,
		query_port: u16,
	},
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Conditioner {
	/// One way latency in milliseconds
	pub latency_ms: u16,
	/// One way jitter in milliseconds
	pub jitter_ms: u16,
	/// Percentage of packet loss
	pub packet_loss: f32,
}

impl Conditioner {
	pub fn build(&self) -> LinkConditionerConfig {
		LinkConditionerConfig {
			incoming_latency: Duration::from_millis(self.latency_ms as u64),
			incoming_jitter: Duration::from_millis(self.jitter_ms as u64),
			incoming_loss: self.packet_loss,
		}
	}
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ServerSettings {
	/// If true, disable any rendering-related plugins
	pub headless: bool,

	/// If true, enable bevy_inspector_egui
	pub inspector: bool,

	/// Possibly add a conditioner to simulate network conditions
	pub conditioner: Option<Conditioner>,

	/// Which transport to use
	pub transport: Vec<ServerTransports>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ClientSettings {
	/// If true, enable bevy_inspector_egui
	pub inspector: bool,

	/// The client id
	pub client_id: u64,

	/// The client port to listen on
	pub client_port: u16,

	/// The ip address of the server
	pub server_addr: Ipv4Addr,

	/// The port of the server
	pub server_port: u16,

	/// Which transport to use
	pub transport: ClientTransports,

	/// Possibly add a conditioner to simulate network conditions
	pub conditioner: Option<Conditioner>,
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
pub struct SharedSettings {
	/// An id to identify the protocol version
	pub protocol_id: u64,

	/// a 32-byte array to authenticate via the Netcode.io protocol
	pub private_key: [u8; 32],
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Settings {
	pub server: ServerSettings,
	pub client: ClientSettings,
	pub shared: SharedSettings,
}
#[cfg(feature = "lightyear/webtransport")]
impl Default for Settings {
	fn default() -> Self {
		Self {
					client: ClientSettings{
							inspector: true,
							client_id: 0,
							client_port: 0, // the OS will assign a random open port
							server_addr: [127,0,0,1].into(),
							conditioner: Some(Conditioner{
									latency_ms: 200,
									jitter_ms: 20,
									packet_loss: 0.05
					}),
							server_port: 5000,
							transport: crate::prelude::ClientTransports::WebTransport{
									// this is only needed for wasm, the self-signed certificates are only valid for 2 weeks
									// the server will print the certificate digest on startup
									certificate_digest: "a71e935276ac0e37db88bfa702b3ece66f30b3ad4f6117c12b782fe0f2817f9f".into(),
							},
							// server_port: 5001,
							// transport: Udp,
							// server_port: 5002,
							// transport: WebSocket,
							// server_port: 5003,
							// transport: Steam(
							//     app_id: 480,
							// )
							},
					server: ServerSettings{
							headless: true,
							inspector: false,
							conditioner: Some(Conditioner{
									latency_ms: 200,
									jitter_ms: 20,
									packet_loss: 0.05
					}),
							transport: vec![
								crate::prelude::ServerTransports::WebTransport{
											local_port: 5000
									},
									crate::prelude::ServerTransports::Udp{
											local_port: 5001
									},
									crate::prelude::ServerTransports::WebSocket{
											local_port: 5002
									},
									// Steam(
									//     app_id: 480,
									//     server_ip: "0.0.0.0",
									//     game_port: 5003,
									//     query_port: 27016,
									// ),
							],
									},
					shared: SharedSettings{
							protocol_id: 0,
							private_key: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
					}
				}
	}
}



pub fn build_server_netcode_config(
	conditioner: Option<&Conditioner>,
	shared: &SharedSettings,
	transport_config: TransportConfig,
) -> server::NetConfig {
	let conditioner = conditioner.map_or(None, |c| {
		Some(LinkConditionerConfig {
			incoming_latency: Duration::from_millis(c.latency_ms as u64),
			incoming_jitter: Duration::from_millis(c.jitter_ms as u64),
			incoming_loss: c.packet_loss,
		})
	});
	let netcode_config = server::NetcodeConfig::default()
		.with_protocol_id(shared.protocol_id)
		.with_key(shared.private_key);
	let io_config = IoConfig::from_transport(transport_config);
	let io_config = if let Some(conditioner) = conditioner {
		io_config.with_conditioner(conditioner)
	} else {
		io_config
	};
	server::NetConfig::Netcode {
		config: netcode_config,
		io: io_config,
	}
}

/// Parse the settings into a list of `NetConfig` that are used to configure how the lightyear server
/// listens for incoming client connections
#[cfg(not(target_family = "wasm"))]
pub fn get_server_net_configs(settings: &Settings) -> Vec<server::NetConfig> {
	settings
		.server
		.transport
		.iter()
		.map(|t| match t {
			ServerTransports::Udp { local_port } => {
				build_server_netcode_config(
					settings.server.conditioner.as_ref(),
					&settings.shared,
					TransportConfig::UdpSocket(SocketAddr::new(
						Ipv4Addr::UNSPECIFIED.into(),
						*local_port,
					)),
				)
			}
			#[cfg(feature = "lightyear/webtransport")]
			ServerTransports::WebTransport { local_port } => {
				use async_compat::Compat;
				use bevy::tasks::IoTaskPool;
				use lightyear::prelude::server::Certificate;
				// this is async because we need to load the certificate from io
				// we need async_compat because wtransport expects a tokio reactor
				let certificate = IoTaskPool::get()
					.scope(|s| {
						s.spawn(Compat::new(async {
							Certificate::load(
								"../certificates/cert.pem",
								"../certificates/key.pem",
							)
							.await
							.unwrap()
						}));
					})
					.pop()
					.unwrap();
				let digest =
					&certificate.hashes()[0].to_string().replace(":", "");
				println!(
					"Generated self-signed certificate with digest: {}",
					digest
				);
				build_server_netcode_config(
					settings.server.conditioner.as_ref(),
					&settings.shared,
					TransportConfig::WebTransportServer {
						server_addr: SocketAddr::new(
							Ipv4Addr::UNSPECIFIED.into(),
							*local_port,
						),
						certificate,
					},
				)
			}
			#[cfg(feature = "lightyear/websocket")]
			ServerTransports::WebSocket { local_port } => build_server_netcode_config(
				settings.server.conditioner.as_ref(),
				&settings.shared,
				TransportConfig::WebSocketServer {
					server_addr: SocketAddr::new(
						Ipv4Addr::UNSPECIFIED.into(),
						*local_port,
					),
				},
			),
			#[cfg(feature = "lightyear/steam")]
			ServerTransports::Steam {
				app_id,
				server_ip,
				game_port,
				query_port,
			} => server::NetConfig::Steam {
				config: server::SteamConfig {
					app_id: *app_id,
					server_ip: *server_ip,
					game_port: *game_port,
					query_port: *query_port,
					max_clients: 16,
					version: "1.0".to_string(),
				},
				conditioner: settings
					.server
					.conditioner
					.as_ref()
					.map_or(None, |c| Some(c.build())),
			},
		})
		.collect()
}

/// Build a netcode config for the client
pub fn build_client_netcode_config(
	client_id: u64,
	server_addr: SocketAddr,
	conditioner: Option<&Conditioner>,
	shared: &SharedSettings,
	transport_config: TransportConfig,
) -> client::NetConfig {
	let conditioner = conditioner.map_or(None, |c| Some(c.build()));
	let auth = Authentication::Manual {
		server_addr,
		client_id,
		private_key: shared.private_key,
		protocol_id: shared.protocol_id,
	};
	let netcode_config = client::NetcodeConfig::default();
	let io_config = IoConfig::from_transport(transport_config);
	let io_config = if let Some(conditioner) = conditioner {
		io_config.with_conditioner(conditioner)
	} else {
		io_config
	};
	client::NetConfig::Netcode {
		auth,
		config: netcode_config,
		io: io_config,
	}
}

/// Parse the settings into a `NetConfig` that is used to configure how the lightyear client
/// connects to the server
pub fn get_client_net_config(
	settings: &Settings,
	client_id: u64,
) -> client::NetConfig {
	let server_addr = SocketAddr::new(
		settings.client.server_addr.into(),
		settings.client.server_port,
	);
	let client_addr = SocketAddr::new(
		Ipv4Addr::UNSPECIFIED.into(),
		settings.client.client_port,
	);
	match &settings.client.transport {
		#[cfg(not(target_family = "wasm"))]
		ClientTransports::Udp => build_client_netcode_config(
			client_id,
			server_addr,
			settings.client.conditioner.as_ref(),
			&settings.shared,
			TransportConfig::UdpSocket(client_addr),
		),
		#[allow(unused)]
		#[cfg(feature = "lightyear/webtransport")]
		ClientTransports::WebTransport { certificate_digest } => {
			build_client_netcode_config(
				client_id,
				server_addr,
				settings.client.conditioner.as_ref(),
				&settings.shared,
				TransportConfig::WebTransportClient {
					client_addr,
					server_addr,
					#[cfg(target_family = "wasm")]
					certificate_digest: certificate_digest.clone(),
				},
			)
		}
		#[cfg(feature = "lightyear/websocket")]
		ClientTransports::WebSocket => build_client_netcode_config(
			client_id,
			server_addr,
			settings.client.conditioner.as_ref(),
			&settings.shared,
			TransportConfig::WebSocketClient { server_addr },
		),
		#[cfg(not(target_family = "wasm"))]
		#[cfg(feature = "lightyear/steam")]
		ClientTransports::Steam { app_id } => client::NetConfig::Steam {
			config: SteamConfig {
				server_addr,
				app_id: *app_id,
			},
			conditioner: settings
				.server
				.conditioner
				.as_ref()
				.map_or(None, |c| Some(c.build())),
		},
	}
}
