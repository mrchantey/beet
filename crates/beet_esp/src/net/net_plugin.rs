use crate::prelude::*;
use anyhow::Result;
use bevy::prelude::*;
use esp_idf_hal::task::block_on;


pub struct NetPlugin {
	wifi: WifiClient,
	ws: EspWsClient,
}

impl NetPlugin {
	pub fn new_blocking(wifi: WifiClient, ws: EspWsClient) -> Result<Self> {
		block_on(Self::new_async(wifi, ws))
	}
	pub async fn new_async(
		mut wifi: WifiClient,
		ws: EspWsClient,
	) -> Result<Self> {
		wifi.connect().await?;
		// let mut ws = EspWsClient::new()?;
		// ws.ws.await_upgrade().await?;
		Ok(Self { wifi, ws })
	}

	pub fn non_send_plugin(self, app: &mut App) {
		// let inbox_non_send =
		// 	app.world.non_send_resource::<InboxNonSend>().clone();

		// self.ws
		// 	.add_listener(move |msg| inbox_non_send.push(msg.clone()));

		// app.add_systems(
		// 	Update,
		// 	message_outbox_service.in_set(MessagePostUpdateSet),
		// );
		app.insert_non_send_resource(self.wifi);
		app.insert_non_send_resource(self.ws);
	}
}

// fn message_outbox_service(
// 	mut outbox: ResMut<Outbox>,
// 	mut ws_client: NonSendMut<EspWsClient>,
// ) {
// 	let messages = std::mem::replace(outbox.as_mut(), Outbox::default());
// 	for item in messages.0.into_iter() {
// 		if let Err(err) = block_on(ws_client.send(item.clone())) {
// 			log::error!("Failed to send message: {:?}", err);
// 		}
// 	}
// }
