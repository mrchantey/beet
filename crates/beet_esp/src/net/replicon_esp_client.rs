use super::ws_client::WsClient;
use beet_net::message::MessageReplicon;
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use forky_core::ResultTEExt;

pub struct RepliconEspClientPlugin;

impl Plugin for RepliconEspClientPlugin {
	fn build(&self, app: &mut App) {
		// app.configure_sets(
		// 	PreUpdate,
		// ClientSet::ReceivePackets.after(QuinnetSyncUpdate),
		// )
		app.add_systems(
			PreUpdate,
			(
				sync_connection_status,
				receive_packets.run_if(client_connected),
			)
				.chain()
				.in_set(ClientSet::ReceivePackets),
		)
		.add_systems(
			PostUpdate,
			send_packets
				.in_set(ClientSet::SendPackets)
				.run_if(client_connected),
		);
	}
}

fn sync_connection_status(
	ws_client: NonSend<WsClient>,
	mut replicon_client: ResMut<RepliconClient>,
) {
	let new_status =
		match (ws_client.ws.is_connected(), replicon_client.status()) {
			(false, RepliconClientStatus::Disconnected) => {
				RepliconClientStatus::Connecting
			}
			(false, RepliconClientStatus::Connected { .. }) => {
				// we'll get one frame of disconnected
				RepliconClientStatus::Disconnected
			}
			(true, _) => RepliconClientStatus::Connected { client_id: None },
			_ => replicon_client.status(),
		};
	if replicon_client.status() != new_status {
		log::info!("Replicon client status: {:?}", new_status);
		replicon_client.set_status(new_status);
	}
}

fn client_connected(ws_client: NonSend<WsClient>) -> bool {
	ws_client.ws.is_connected()
}

fn receive_packets(
	ws_client: NonSendMut<WsClient>,
	mut replicon_client: ResMut<RepliconClient>,
) {
	while let Ok(bytes) = ws_client.try_recv() {
		MessageReplicon::bytes_to_client(&mut replicon_client, &bytes)
			.ok_or(|e| log::error!("{e}"));
	}
}

fn send_packets(
	mut ws_client: NonSendMut<WsClient>,
	mut replicon_client: ResMut<RepliconClient>,
) {
	if let Some(bytes) =
		MessageReplicon::bytes_from_client(&mut replicon_client)
			.ok_or(|e| log::error!("{e}"))
	{
		ws_client.send(&bytes).ok_or(|e| log::error!("{e}"));
	}
}
