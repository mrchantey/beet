//! This module contains the shared code between the client and the server.
//!
//! The rendering code is here because you might want to run the example in host-server mode, where the server also acts as a client.
//! The simulation logic (movement, etc.) should be shared between client and server to guarantee that there won't be
//! mispredictions/rollbacks.
use crate::lightyear::protocol::*;
use bevy::prelude::*;
use bevy::utils::Duration;
use lightyear::prelude::*;
use lightyear::shared::config::Mode;

pub fn shared_config(mode: Mode) -> SharedConfig {
	SharedConfig {
		client_send_interval: Duration::default(),
		server_send_interval: Duration::from_millis(40),
		// server_send_interval: Duration::from_millis(100),
		tick: TickConfig {
			tick_duration: Duration::from_secs_f64(1.0 / 64.0),
		},
		mode,
	}
}

pub struct SharedPlugin;

impl Plugin for SharedPlugin {
	fn build(&self, app: &mut App) {
		// the protocol needs to be shared between the client and server
		app.add_plugins(ProtocolPlugin);
	}
}
