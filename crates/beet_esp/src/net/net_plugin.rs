// use crate::prelude::*;
// use anyhow::Result;
// use bevy::prelude::*;
// use esp_idf_hal::task::block_on;

// pub struct NetPlugin {
// 	wifi: WifiClient,
// 	ws: WsClient,
// }

// impl NetPlugin {
// 	pub fn new_blocking(wifi: WifiClient, ws: WsClient) -> Result<Self> {
// 		block_on(Self::new_async(wifi, ws))
// 	}
// 	pub async fn new_async(mut wifi: WifiClient, ws: WsClient) -> Result<Self> {
// 		wifi.connect().await?;
// 		// let mut ws = EspWsClient::new()?;
// 		// ws.ws.await_upgrade().await?;
// 		Ok(Self { wifi, ws })
// 	}

// 	pub fn non_send_plugin(self, app: &mut App) {
// 		app /*-*/
// 			.insert_non_send_resource(self.ws)
// 			.insert_non_send_resource(self.wifi);
// 	}
// }
