use super::native_client::NativeWsClient;
use crate::prelude::*;
use bevy::prelude::*;
use bevy::tasks::block_on;
use bevy::time::common_conditions::on_timer;
use forky_core::ResultTEExt;
use std::sync::Arc;
use std::sync::Mutex;


#[derive(Default, Clone, Resource, Deref, DerefMut)]
pub struct OutgoingMessageCollector(pub Vec<Message>);

#[derive(Clone, Resource, Deref)]
pub struct ArcNativeClient(pub Arc<Mutex<NativeWsClient>>);

impl ArcNativeClient {
	pub fn new(client: NativeWsClient) -> Self {
		Self(Arc::new(Mutex::new(client)))
	}
}

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


			let client = ArcNativeClient::new(client);

			app.insert_resource(client)
				.insert_resource(OutgoingMessageCollector::default())
				.add_systems(
					Update,
					handle_incoming
						.run_if(on_timer(SOCKET_INTERVAL))
						.before(MessageSet),
				)
				.add_systems(
					Update,
					collect_outgoing_events
						.run_if(on_event::<MessageOutgoing>())
						.after(MessageSet),
				)
				.add_systems(
					Update,
					handle_outgoing
						.run_if(on_timer(SOCKET_INTERVAL))
						.after(MessageSet)
						.after(collect_outgoing_events),
				);
		}
	}
}

fn collect_outgoing_events(
	mut events: EventReader<MessageOutgoing>,
	mut collector: ResMut<OutgoingMessageCollector>,
) {
	let messages = events
		.read()
		.map(|msg| {
			log::info!(">>> MESSAGE: {:?}", msg);
			msg.0.clone()
		})
		.collect::<Vec<_>>();
	// if let Some(bytes) =
	// 	Message::into_bytes(messages).ok_or(|e| log::error!("{e}"))
	// {
	collector.extend(messages);
	// 	collector.extend_from_slice(&bytes);
	// }
}


fn handle_incoming(
	mut events: EventWriter<MessageIncoming>,
	client: Res<ArcNativeClient>,
) {
	let mut client = client.0.lock().unwrap();
	if let Some(messages) = client.recv().ok_or(|e| log::error!("foo {e}")) {
		for message in messages {
			log::info!("<<< MESSAGE: {:?}", message);
			events.send(MessageIncoming(message));
		}
	}
}


fn handle_outgoing(
	mut events: ResMut<OutgoingMessageCollector>,
	client: Res<ArcNativeClient>,
) {
	if events.is_empty() {
		return;
	}

	let messages = std::mem::take(&mut events.0);

	if let Some(bytes) =
		Message::into_bytes(&messages).ok_or(|e| log::error!("{e}"))
	{
		let client = client.clone();
		std::thread::spawn(|| {
			block_on(async move {
				client
					.lock()
					.unwrap()
					.send_bytes(bytes)
					.await
					.ok_or(|e| log::error!("{e}"))
			});
		});
	}
}
