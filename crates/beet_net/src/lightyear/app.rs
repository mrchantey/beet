use crate::lightyear::*;
use bevy::input::InputPlugin;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use lightyear::client::plugin::ClientPlugins;
use lightyear::prelude::client;
use lightyear::prelude::client::ClientConfig;
use lightyear::prelude::server;
use lightyear::prelude::server::ServerTransport;
use lightyear::prelude::*;
use lightyear::server::config::ServerConfig;
use lightyear::server::plugin::ServerPlugins;
use lightyear::transport::LOCAL_SOCKET;
use std::time::Duration;


fn settings() -> Settings {
	let settings_str = include_str!("../../assets/settings.ron");
	let settings = ron::de::from_str::<Settings>(settings_str).unwrap();
	settings
}

pub fn client_app() -> App {
	let settings = settings();

	let mut app = App::new();
	app.add_plugins((
		LogPlugin::default(),
		MinimalPlugins,
		InputPlugin,
		ClientPlugins::new(client_config(settings, Some(1))),
		BeetProtocolPlugin,
		ExampleClientPlugin,
	));
	app
}

pub fn server_app() -> App {
	let settings = settings();

	let mut app = App::new();
	app.add_plugins((
		LogPlugin::default(),
		MinimalPlugins,
		ServerPlugins::new(server_config(settings.clone(), vec![])),
		ExampleServerPlugin,
		ClientPlugins::new(client_config(settings, Some(0))),
		ExampleClientPlugin,
		BeetProtocolPlugin,
	));
	app
}

pub fn loopback_apps() -> (App, App) {
	let settings = settings();

	let (server_config, client_config) = loopback_configs(settings);
	let mut server_app = App::new();
	server_app.add_plugins((
		LogPlugin::default(),
		MinimalPlugins,
		ClientPlugins::new(client_config.clone()),
		ExampleClientPlugin,
		ServerPlugins::new(server_config),
		ExampleServerPlugin,
		BeetProtocolPlugin,
	));
	let mut client_app = App::new();
	client_app.add_plugins((
		LogPlugin::default(),
		MinimalPlugins,
		InputPlugin,
		ClientPlugins::new(client_config),
		ExampleClientPlugin,
		BeetProtocolPlugin,
	));
	(server_app, client_app)
}

pub fn client_config(
	settings: Settings,
	client_id: Option<u64>,
) -> ClientConfig {
	// use the cli-provided client id if it exists, otherwise use the settings client id
	let client_id = client_id.unwrap_or(settings.client.client_id);
	let net_config = get_client_net_config(&settings, client_id);
	client::ClientConfig {
		shared: shared_config(Mode::Separate),
		net: net_config,
		..default()
	}
}

pub fn server_config(
	settings: Settings,
	extra_transport_configs: Vec<ServerTransport>,
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
	ServerConfig {
		shared: shared_config(Mode::Separate),
		net: net_configs,
		..default()
	}
}

pub fn loopback_configs(settings: Settings) -> (ServerConfig, ClientConfig) {
	// create client app
	let (from_server_send, from_server_recv) = crossbeam_channel::unbounded();
	let (to_server_send, to_server_recv) = crossbeam_channel::unbounded();
	// we will communicate between the client and server apps via channels
	let client_transport = client::ClientTransport::LocalChannel {
		recv: from_server_recv,
		send: to_server_send,
	};
	let net_config = build_client_netcode_config(
		settings.client.client_id,
		// when communicating via channels, we need to use the address `LOCAL_SOCKET` for the server
		LOCAL_SOCKET,
		settings.client.conditioner.as_ref(),
		&settings.shared,
		client_transport,
	);
	let client_config = ClientConfig {
		shared: shared_config(Mode::Separate),
		net: net_config,
		..default()
	};

	let extra_transport_configs = vec![server::ServerTransport::Channels {
		// even if we communicate via channels, we need to provide a socket address for the client
		channels: vec![(LOCAL_SOCKET, to_server_recv, from_server_send)],
	}];

	let server_config = server_config(settings, extra_transport_configs);

	(server_config, client_config)
}



pub const FIXED_TIMESTEP_HZ: f64 = 64.0;

/// The [`SharedConfig`] must be shared between the `ClientConfig` and `ServerConfig`
pub fn shared_config(mode: Mode) -> SharedConfig {
	SharedConfig {
		client_send_interval: Duration::default(),
		// send an update every 100ms
		server_send_interval: Duration::from_millis(100),
		tick: TickConfig {
			tick_duration: Duration::from_secs_f64(1.0 / FIXED_TIMESTEP_HZ),
		},
		mode,
	}
}
