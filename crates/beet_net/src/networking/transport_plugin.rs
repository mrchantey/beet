use crate::prelude::*;
use bevy::prelude::*;
use bevy::tasks::block_on;
use bevy::time::common_conditions::on_timer;
use forky_core::ResultTEExt;
use std::sync::Arc;
use std::sync::Mutex;


#[derive(Clone, Resource, Deref, DerefMut)]
pub struct TransportClient<T: Transport>(pub T);

impl<T: Transport> TransportClient<T> {
	pub fn new(client: T) -> Self { Self(client) }
}

pub struct TransportPlugin<T: Transport> {
	pub transport: T,
}

impl<T: Transport> TransportPlugin<T> {
	pub fn new(transport: T) -> Self { Self { transport } }
	pub fn arc(transport: T) -> TransportPlugin<Arc<Mutex<T>>> {
		TransportPlugin::new(Arc::new(Mutex::new(transport)))
	}
}
/// Adds the [`transport_incoming`] and [`transport_outgoing`] systems
impl<T: SendTransport> Plugin for TransportPlugin<T> {
	fn build(&self, app: &mut App) {
		app.insert_resource(TransportClient(self.transport.clone()))
			.add_systems(
				Update,
				transport_incoming::<T>
					.run_if(on_timer(SOCKET_INTERVAL))
					.before(MessageIncomingSet),
			)
			.add_systems(
				Update,
				transport_outgoing::<T>
					.run_if(on_timer(SOCKET_INTERVAL))
					.after(MessageOutgoingSet),
			);
	}
}


pub(crate) fn transport_incoming<T: SendTransport>(
	mut events: ResMut<MessageIncoming>,
	mut client: ResMut<TransportClient<T>>,
) {
	if let Some(messages) = client.recv().ok_or(|e| log::error!("foo {e}")) {
		for message in messages {
			log::info!("<<< MESSAGE: {:?}", message);
			events.push(message);
		}
	}
}


pub(crate) fn transport_outgoing<T: SendTransport>(
	mut outgoing: ResMut<MessageOutgoing>,
	client: ResMut<TransportClient<T>>,
) {
	if outgoing.is_empty() {
		return;
	}

	let messages = outgoing.drain(..).collect();
	let mut client = client.clone();
	{
		//TODO transport defines async runtime
		std::thread::spawn(|| {
			block_on(async move {
				client.send(&messages).await.ok_or(|e| log::error!("{e}"))
			});
		});
	}
}
