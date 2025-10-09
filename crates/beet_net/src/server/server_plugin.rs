use crate::prelude::*;
use crate::server::run_server;
use beet_core::prelude::*;



pub struct ServerPlugin;

impl Plugin for ServerPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin(AsyncPlugin)
			.init_resource::<ServerSettings>()
			.init_resource::<ServerStatus>()
			.add_systems(Startup, run_server);
	}
}

#[derive(Resource)]
pub struct ServerSettings {
	pub port: u16,
}

impl Default for ServerSettings {
	fn default() -> Self {
		Self {
			port: DEFAULT_SERVER_PORT,
		}
	}
}

#[derive(Default, Resource)]
pub struct ServerStatus {
	request_count: u128,
}
impl ServerStatus {
	pub fn num_requests(&self) -> u128 { self.request_count }
	pub(super) fn increment_requests(&mut self) -> &mut Self {
		self.request_count += 1;
		self
	}
}
