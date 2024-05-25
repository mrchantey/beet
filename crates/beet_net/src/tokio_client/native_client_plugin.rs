use super::native_client::NativeWsClient;
use crate::prelude::*;
use bevy::prelude::*;
use bevy::tasks::block_on;
use forky_core::ResultTEExt;

pub struct NativeClientPlugin {
	pub address: String,
}

impl Default for NativeClientPlugin {
	fn default() -> Self {
		Self {
			address: "ws://127.0.0.1:3000/ws".into(),
		}
	}
}

impl Plugin for NativeClientPlugin {
	fn build(&self, app: &mut App) {
		// TODO async tasks
		if let Some(client) =
			block_on(async { NativeWsClient::new(&self.address).await })
				.ok_or(|e| log::error!("{e}"))
		{
			log::info!("client connected");

			app.add_transport(client);

			// app.add_plugins(TransportPlugin::arc(client));
		}
	}
}
