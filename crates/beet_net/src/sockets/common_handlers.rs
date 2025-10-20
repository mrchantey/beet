use std::time::SystemTime;

use super::Message;
use super::*;
use beet_core::prelude::*;

/// Log the received socket message to info output
pub fn log_recv(ev: On<MessageRecv>) {
	info!("Socket {:#?}", ev.event());
}
/// Log the sent socket message to info output
pub fn log_send(ev: On<MessageSend>) {
	info!("Socket {:#?}", ev.event());
}

/// Return the exact same Text/Binary message back to sender.
/// Ping/Pong/Close messages are ignored.
pub fn echo_message(ev: On<MessageRecv>, mut commands: Commands) {
	match ev.event().inner() {
		Message::Ping(_) => {}
		Message::Pong(_) => {}
		Message::Close(_) => {}
		message => {
			commands
				.entity(ev.original_target())
				.trigger_target(MessageSend(message.clone()));
		}
	}
}
/// Sockets must acknowledge a Close with one of their own, use this
/// if the receiver does not have an opinion about the [`CloseFrame`].
pub fn echo_close(ev: On<MessageRecv>, mut commands: Commands) {
	match ev.event().inner() {
		Message::Close(payload) => {
			commands
				.entity(ev.original_target())
				.trigger_target(MessageSend(Message::Close(payload.clone())));
		}
		_ => {}
	}
}



#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PingTime {
	timestamp: SystemTime,
}

impl Default for PingTime {
	fn default() -> Self {
		Self {
			timestamp: SystemTime::now(),
		}
	}
}

impl PingTime {
	pub fn into_message(&self) -> Message {
		let ping_time = serde_json::to_vec(&self).unwrap();
		Message::Ping(ping_time.into())
	}
}


/// On receiving a [`Pong`] message which contains a serialized [`PingTime`],
/// calculate and log the round trip time.
pub fn echo_pingtime(ev: On<MessageRecv>) {
	match ev.event().inner() {
		// 1. print the round trip duration and send a text message
		Message::Pong(payload) => {
			let Ok(ping_time) = serde_json::from_slice::<PingTime>(&payload)
			else {
				return;
			};
			let now = SystemTime::now();
			let rtt = now.duration_since(ping_time.timestamp).unwrap();
			info!(
				"Round Trip Time: {}",
				time_ext::pretty_print_duration(rtt)
			);
		}
		_ => {}
	}
}
