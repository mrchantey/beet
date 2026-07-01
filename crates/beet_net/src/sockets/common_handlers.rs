use super::Message;
use super::*;
use beet_core::prelude::*;
use bevy::platform::sync::OnceLock;

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

/// Microseconds elapsed on a process-global monotonic clock, the cross-platform
/// (no_std) serializable stand-in for a wall-clock timestamp.
fn ping_epoch_micros() -> u64 {
	static EPOCH: OnceLock<Instant> = OnceLock::new();
	EPOCH.get_or_init(Instant::now).elapsed().as_micros() as u64
}

/// A timestamp payload for WebSocket ping/pong round-trip time measurement.
///
/// Carries microseconds from a process-global monotonic [`Instant`] epoch (a
/// serializable `u64`), so the round trip is measured against beet's
/// cross-platform clock rather than the std-only `SystemTime`.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PingTime {
	elapsed_micros: u64,
}

impl Default for PingTime {
	fn default() -> Self {
		Self {
			elapsed_micros: ping_epoch_micros(),
		}
	}
}

#[cfg(feature = "json")]
impl PingTime {
	/// Converts this ping time into a WebSocket [`Message::Ping`].
	pub fn into_message(&self) -> Message {
		let ping_time = serde_json::to_vec(&self).unwrap();
		Message::Ping(ping_time.into())
	}
}

/// On receiving a [`Pong`] message which contains a serialized [`PingTime`],
/// calculate and log the round trip time.
#[cfg(feature = "json")]
pub fn echo_pingtime(ev: On<MessageRecv>) {
	match ev.event().inner() {
		// 1. print the round trip duration and send a text message
		Message::Pong(payload) => {
			let Ok(ping_time) = serde_json::from_slice::<PingTime>(&payload)
			else {
				return;
			};
			let rtt =
				ping_epoch_micros().saturating_sub(ping_time.elapsed_micros);
			info!(
				"Round Trip Time: {}",
				time_ext::pretty_print_duration(Duration::from_micros(rtt))
			);
		}
		_ => {}
	}
}
