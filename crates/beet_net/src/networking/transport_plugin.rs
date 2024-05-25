use crate::prelude::*;
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use forky_core::ResultTEExt;
use std::time::Duration;

pub const DEFAULT_TRANSPORT_INTERVAL: Duration = Duration::from_millis(100);

#[extend::ext(name=AppExtTransport)]
pub impl App {
	/// Adds the [`transport_incoming`] and [`transport_outgoing`] systems for a given transport type, and inserts it as a [`NonSend`]
	/// ```rust
	/// app
	/// 	.insert_non_send_resource(ChannelsTransport)
	/// 	.add_plugins(TransportPlugin::<ChannelsTransport>::new());
	/// ```
	fn add_transport<T: 'static + Transport>(
		&mut self,
		transport: T,
	) -> &mut Self {
		self.add_transport_with_duration(transport, DEFAULT_TRANSPORT_INTERVAL)
	}
	fn add_transport_with_duration<T: 'static + Transport>(
		&mut self,
		transport: T,
		interval: Duration,
	) -> &mut Self {
		self.insert_non_send_resource(transport).add_systems(
			Update,
			(
				transport_incoming::<T>
					.run_if(on_timer(interval))
					.before(MessageIncomingSet),
				transport_outgoing::<T>
					.run_if(on_timer(interval))
					.after(MessageOutgoingSet),
			),
		);
		self
	}
}

pub(crate) fn transport_incoming<T: Transport>(
	mut events: ResMut<MessageIncoming>,
	mut transport: NonSendMut<T>,
) {
	if let Some(messages) = transport.recv().ok_or(|e| log::error!("foo {e}")) {
		for message in messages {
			log::info!("<<< MESSAGE: {:?}", message);
			events.push(message);
		}
	}
}

pub(crate) fn transport_outgoing<T: Transport>(
	mut outgoing: ResMut<MessageOutgoing>,
	mut transport: NonSendMut<T>,
) {
	if outgoing.is_empty() {
		return;
	}

	let messages = outgoing.drain(..).collect();
	transport.send(&messages).ok_or(|e| log::error!("{e}"));
	// {
	// 	#[cfg(target_arch = "wasm32")]
	// 	wasm_bindgen_futures::spawn_local(async move {
	// 	});

	// 	#[cfg(not(target_arch = "wasm32"))]
	// 	//TODO transport defines async runtime
	// 	std::thread::spawn(|| {
	// 		bevy::tasks::block_on(async move {
	// 			client.send(&messages).await.ok_or(|e| log::error!("{e}"));
	// 		});
	// 	});
	// }
}

// #[derive(Debug, Default, Copy, Clone)]
// pub struct TransportPlugin<T: Transport> {
// 	phantom: PhantomData<T>,
// 	interval: Duration,
// }

// impl<T: Transport> TransportPlugin<T> {
// 	pub fn new() -> Self {
// 		Self {
// 			phantom: PhantomData,
// 			interval: DEFAULT_TRANSPORT_INTERVAL,
// 		}
// 	}
// 	pub fn new_with_interval(interval: Duration) -> Self {
// 		Self {
// 			phantom: PhantomData,
// 			interval,
// 		}
// 	}

// 	pub fn with_interval(mut self, interval: Duration) -> Self {
// 		self.interval = interval;
// 		self
// 	}
// }
