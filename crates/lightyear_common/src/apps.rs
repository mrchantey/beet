//! This example showcases how to use Lightyear with Bevy, to easily get replication along with prediction/interpolation working.
//!
//! There is a lot of setup code, but it's mostly to have the examples work in all possible configurations of transport.
//! (all transports are supported, as well as running the example in listen-server or host-server mode)
//!
//!
//! Run with
//! - `cargo run -- server`
//! - `cargo run -- client -c 1`
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

use crate::settings::*;
use crate::shared::shared_config;
use bevy::app::Plugins;
use bevy::log::Level;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use clap::Parser;
use clap::ValueEnum;
use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;
use lightyear::prelude::client;
use lightyear::prelude::client::ClientConfig;
use lightyear::prelude::server;
use lightyear::prelude::*;
use lightyear::server::config::ServerConfig;
use lightyear::shared::log::add_log_layer;
use lightyear::transport::LOCAL_SOCKET;
use serde::Deserialize;
use serde::Serialize;
use std::net::SocketAddr;
use std::str::FromStr;

#[derive(Parser, PartialEq, Debug)]
pub enum Cli {
	/// We have the client and the server running inside the same app.
	/// The server will also act as a client.
	#[cfg(not(target_family = "wasm"))]
	HostServer {
		#[arg(short, long, default_value = None)]
		client_id: Option<u64>,
	},
	#[cfg(not(target_family = "wasm"))]
	/// We will create two apps: a client app and a server app.
	/// Data gets passed between the two via channels.
	ListenServer {
		#[arg(short, long, default_value = None)]
		client_id: Option<u64>,
	},
	#[cfg(not(target_family = "wasm"))]
	/// Dedicated server
	Server,
	/// The program will act as a client
	Client {
		#[arg(short, long, default_value = None)]
		client_id: Option<u64>,
	},
}

/// Pars the CLI arguments.
/// `clap` doesn't run in wasm; we simply run in Client mode with a random ClientId
pub fn cli() -> Cli {
	cfg_if::cfg_if! {
		if #[cfg(target_family = "wasm")] {
			let client_id = rand::random::<u64>();
			Cli::Client {
				client_id: Some(client_id)
			}
		} else {
			Cli::parse()
		}
	}
}

/// Apps that will be returned from the `build_apps` function
pub enum Apps {
	/// A single app that contains only the ClientPlugins
	Client { app: App, config: ClientConfig },
	/// A single app that contains only the ServerPlugins
	Server { app: App, config: ServerConfig },
	/// Two apps (Client and Server) that will run in separate threads
	ListenServer {
		client_app: App,
		client_config: ClientConfig,
		server_app: App,
		server_config: ServerConfig,
	},
	/// A single app that contains both the Client and Server plugins
	HostServer {
		app: App,
		client_config: ClientConfig,
		server_config: ServerConfig,
	},
}

impl Apps {
	/// Build the bevy App(s).
	pub fn from_cli(settings: Settings, cli: Cli) -> Apps {
		match cli {
			// ListenServer using a single app
			#[cfg(not(target_family = "wasm"))]
			Cli::HostServer { client_id } => {
				let client_net_config = client::NetConfig::Local {
					id: client_id.unwrap_or(settings.client.client_id),
				};
				let (client_config, server_config) =
					settings_to_hostserver_config(
						settings,
						vec![],
						client_net_config,
					);
				Apps::host_server(client_config, server_config)
			}
			#[cfg(not(target_family = "wasm"))]
			Cli::ListenServer { client_id } => {
				// create client app
				let (from_server_send, from_server_recv) =
					crossbeam_channel::unbounded();
				let (to_server_send, to_server_recv) =
					crossbeam_channel::unbounded();
				// we will communicate between the client and server apps via channels
				let transport_config = client::ClientTransport::LocalChannel {
					recv: from_server_recv,
					send: to_server_send,
				};
				let net_config = build_client_netcode_config(
					client_id.unwrap_or(settings.client.client_id),
					// when communicating via channels, we need to use the address `LOCAL_SOCKET` for the server
					LOCAL_SOCKET,
					settings.client.conditioner.as_ref(),
					&settings.shared,
					transport_config,
				);
				let client_config =
					settings_to_client_config(settings.clone(), net_config);

				// create server app
				let extra_transport_configs =
					vec![server::ServerTransport::Channels {
						// even if we communicate via channels, we need to provide a socket address for the client
						channels: vec![(
							LOCAL_SOCKET,
							to_server_recv,
							from_server_send,
						)],
					}];
				let server_config = settings_to_server_config(
					settings,
					extra_transport_configs,
				);
				Apps::listen_server(client_config, server_config)
			}
			#[cfg(not(target_family = "wasm"))]
			Cli::Server => {
				let config = settings_to_server_config(settings, vec![]);
				Apps::server(config)
			}
			Cli::Client { client_id } => {
				let server_addr = SocketAddr::new(
					settings.client.server_addr.into(),
					settings.client.server_port,
				);
				// use the cli-provided client id if it exists, otherwise use the settings client id
				let client_id = client_id.unwrap_or(settings.client.client_id);
				let net_config = get_client_net_config(&settings, client_id);
				let config = settings_to_client_config(settings, net_config);
				Apps::client(config)
			}
		}
	}


	pub fn from_cli_crossbeam(
		settings: Settings,
		cli: Cli,
		send: Sender<Vec<u8>>,
		recv: Receiver<Vec<u8>>,
	) -> Self {
		match cli {
			Cli::HostServer { .. } => {
				panic!("crossbeam apps not supported for HostServer")
			}
			Cli::ListenServer { .. } => {
				panic!("crossbeam apps not supported for ListenServer")
			}
			Cli::Server => {
				let config =
					settings_to_server_config_crossbeam(settings, send, recv);
				Apps::server(config)
			}
			Cli::Client { client_id } => {
				let config =
					build_crossbeam_client_app(settings, send, recv, client_id);
				Apps::client(config)
			}
		}
	}

	pub fn client(config: ClientConfig) -> Self {
		let mut app = App::new();
		app.add_plugins(client::ClientPlugins {
			config: config.clone(),
		});
		Self::Client { app, config }
	}

	pub fn server(config: ServerConfig) -> Self {
		let mut app = App::new();
		app.add_plugins(server::ServerPlugins {
			config: config.clone(),
		});
		Self::Server { app, config }
	}


	pub fn listen_server(
		client_config: ClientConfig,
		server_config: ServerConfig,
	) -> Self {
		let mut client_app = App::new();
		client_app.add_plugins(client::ClientPlugins {
			config: client_config.clone(),
		});
		let mut server_app = App::new();
		server_app.add_plugins(server::ServerPlugins {
			config: server_config.clone(),
		});
		Self::ListenServer {
			client_app,
			client_config,
			server_app,
			server_config,
		}
	}

	pub fn host_server(
		client_config: ClientConfig,
		server_config: ServerConfig,
	) -> Self {
		let mut app = App::new();
		app.add_plugins(client::ClientPlugins {
			config: client_config.clone(),
		});
		app.add_plugins(server::ServerPlugins {
			config: server_config.clone(),
		});
		Self::HostServer {
			app,
			client_config,
			server_config,
		}
	}

	pub fn add_server_plugins<M>(
		&mut self,
		plugins: impl Plugins<M>,
	) -> &mut Self {
		match self {
			Apps::Server { app, .. } => {
				app.add_plugins(plugins);
			}
			Apps::ListenServer { server_app, .. } => {
				server_app.add_plugins(plugins);
			}
			Apps::HostServer { app, .. } => {
				app.add_plugins(plugins);
			}
			_ => {}
		}
		self
	}

	pub fn add_client_plugins<M>(
		&mut self,
		plugins: impl Plugins<M>,
	) -> &mut Self {
		match self {
			Apps::Client { app, .. } => {
				app.add_plugins(plugins);
			}
			Apps::ListenServer { client_app, .. } => {
				client_app.add_plugins(plugins);
			}
			Apps::HostServer { app, .. } => {
				app.add_plugins(plugins);
			}
			_ => {}
		}
		self
	}
	pub fn add_shared_plugins<M>(
		&mut self,
		plugins: impl Plugins<M> + Clone,
	) -> &mut Self {
		match self {
			Apps::Client { app, .. } => {
				app.add_plugins(plugins);
			}
			Apps::Server { app, .. } => {
				app.add_plugins(plugins);
			}
			Apps::ListenServer {
				client_app,
				server_app,
				..
			} => {
				client_app.add_plugins(plugins.clone());
				server_app.add_plugins(plugins);
			}
			Apps::HostServer { app, .. } => {
				app.add_plugins(plugins);
			}
		}
		self
	}

	/// Add the client, server, and shared plugins to the app
	pub fn add_plugins<M1, M2, M3>(
		&mut self,
		client_plugin: impl Plugins<M1>,
		server_plugin: impl Plugins<M2>,
		shared_plugin: impl Plugins<M3> + Clone,
	) -> &mut Self {
		match self {
			Apps::Client { app, .. } => {
				app.add_plugins((client_plugin, shared_plugin));
			}
			Apps::Server { app, .. } => {
				app.add_plugins((server_plugin, shared_plugin));
			}
			Apps::ListenServer {
				client_app,
				server_app,
				..
			} => {
				client_app.add_plugins((client_plugin, shared_plugin.clone()));
				server_app.add_plugins((server_plugin, shared_plugin));
			}
			Apps::HostServer { app, .. } => {
				app.add_plugins((client_plugin, server_plugin, shared_plugin));
			}
		}
		self
	}

	/// Start running the apps
	pub fn run(&mut self) {
		match self {
			Apps::Client { app, .. } => app.run(),
			Apps::Server { app, .. } => app.run(),
			Apps::ListenServer {
				client_app,
				server_app,
				..
			} => {
				let mut server_app = std::mem::take(server_app);
				std::thread::spawn(move || server_app.run());
				client_app.run();
			}
			Apps::HostServer { app, .. } => {
				app.run();
			}
		}
	}

	pub fn for_each<F: FnMut(&mut App)>(&mut self, mut f: F) {
		match self {
			Apps::Client { app, .. } => f(app),
			Apps::Server { app, .. } => f(app),
			Apps::ListenServer {
				client_app,
				server_app,
				..
			} => {
				f(client_app);
				f(server_app);
			}
			Apps::HostServer { app, .. } => {
				f(app);
			}
		}
	}
}

pub fn settings_to_server_config_crossbeam(
	settings: Settings,
	from_server_send: Sender<Vec<u8>>,
	to_server_recv: Receiver<Vec<u8>>,
) -> ServerConfig {
	let extra_transport_configs = vec![server::ServerTransport::Channels {
		// even if we communicate via channels, we need to provide a socket address for the client
		channels: vec![(LOCAL_SOCKET, to_server_recv, from_server_send)],
	}];
	settings_to_server_config(settings, extra_transport_configs)
}

pub fn build_crossbeam_client_app(
	settings: Settings,
	to_server_send: Sender<Vec<u8>>,
	from_server_recv: Receiver<Vec<u8>>,
	client_id: Option<u64>,
) -> ClientConfig {
	let transport_config = client::ClientTransport::LocalChannel {
		send: to_server_send,
		recv: from_server_recv,
	};
	let net_config = build_client_netcode_config(
		client_id.unwrap_or(settings.client.client_id),
		// when communicating via channels, we need to use the address `LOCAL_SOCKET` for the server
		LOCAL_SOCKET,
		settings.client.conditioner.as_ref(),
		&settings.shared,
		transport_config,
	);
	settings_to_client_config(settings, net_config)
}


/// Build the client app with the `ClientPlugins` added.
/// Takes in a `net_config` parameter so that we configure the network transport.
fn settings_to_client_config(
	settings: Settings,
	net_config: client::NetConfig,
) -> ClientConfig {
	let client_config = client::ClientConfig {
		shared: shared_config(Mode::Separate),
		net: net_config,
		..default()
	};
	client_config
}

/// Build the server app with the `ServerPlugins` added.
#[cfg(not(target_family = "wasm"))]
fn settings_to_server_config(
	settings: Settings,
	extra_transport_configs: Vec<server::ServerTransport>,
) -> ServerConfig {
	// configure the network configuration
	let mut net_configs = get_server_net_configs(&settings);
	let extra_net_configs = extra_transport_configs.into_iter().map(|c| {
		build_server_netcode_config(
			settings.server.conditioner.as_ref(),
			&settings.shared,
			c,
		)
	});
	net_configs.extend(extra_net_configs);
	let server_config = ServerConfig {
		shared: shared_config(Mode::Separate),
		net: net_configs,
		..default()
	};
	server_config
}

/// An `App` that contains both the client and server plugins
#[cfg(not(target_family = "wasm"))]
fn settings_to_hostserver_config(
	settings: Settings,
	extra_transport_configs: Vec<server::ServerTransport>,
	client_net_config: client::NetConfig,
) -> (ClientConfig, ServerConfig) {
	// server config
	let mut net_configs = get_server_net_configs(&settings);
	let extra_net_configs = extra_transport_configs.into_iter().map(|c| {
		build_server_netcode_config(
			settings.server.conditioner.as_ref(),
			&settings.shared,
			c,
		)
	});
	net_configs.extend(extra_net_configs);
	let server_config = server::ServerConfig {
		shared: shared_config(Mode::HostServer),
		net: net_configs,
		..default()
	};

	// client config
	let client_config = client::ClientConfig {
		shared: shared_config(Mode::HostServer),
		net: client_net_config,
		..default()
	};
	(client_config, server_config)
}
