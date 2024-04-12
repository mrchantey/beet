use beet::prelude::*;
use dotenv_codegen::dotenv;
use embedded_svc::ws::FrameType;
use esp_idf_hal::io::EspIOError;
use esp_idf_svc::ws::client::EspWebSocketClient;
use esp_idf_svc::ws::client::EspWebSocketClientConfig;
use esp_idf_svc::ws::client::WebSocketEvent;
use esp_idf_svc::ws::client::WebSocketEventType;
use flume::Sender;
use forky_core::ResultTEExt;
use std::time::Duration;

// 4096 - too big
// 2048 - works, but i think its pretty close
// const CIBORIUM_SCRATCH_BUFFER_SIZE: usize = 4096;

pub type EspWsEvent = WebSocketEvent<'static>;

pub struct EspWsClient {
	pub ws: EspWebSocketClient<'static>,
}

impl EspWsClient {
	pub fn new(mut send: Sender<BeetMessage>) -> anyhow::Result<Self> {
		let timeout = Duration::from_secs(10);
		let config = EspWebSocketClientConfig {
			server_cert: None,
			reconnect_timeout_ms: timeout, //default 10s
			network_timeout_ms: timeout,   //default 10s
			task_stack: 5_000, // 10_000 - not enough heap, default - stack overflow
			..Default::default()
		};

		let url = dotenv!("WS_URL");
		let timeout = Duration::from_secs(10);

		let ws =
			EspWebSocketClient::new(&url, &config, timeout, move |event| {
				parse(event, &mut send).ok_or(|e| log::error!("{e}"));
			})?;

		Ok(Self { ws })
	}

	pub fn send(&mut self, msg: &BeetMessage) -> anyhow::Result<()> {
		let bytes = bincode::serialize(msg)?;
		self.ws.send(FrameType::Binary(false), &bytes)?;
		Ok(())
	}
}

impl Drop for EspWsClient {
	fn drop(&mut self) {
		log::info!("EspWsClient Dropped");
	}
}

fn parse(
	event: &Result<WebSocketEvent<'_>, EspIOError>,
	send: &mut Sender<BeetMessage>,
) -> anyhow::Result<()> {
	match event {
		Ok(event) => {
			match event.event_type {
				WebSocketEventType::Text(_value) => {
					log::error!("Receiving text Socket Messages on ESP32 is not supported");
				}
				WebSocketEventType::Binary(value) => {
					// safer to keep on heap, but watch for framentation
					// let mut scratch_buffer =
					// 	Box::new([0; CIBORIUM_SCRATCH_BUFFER_SIZE]);

					let msg: BeetMessage = bincode::deserialize(&value)?;
					send.send(msg)?;
				}
				_ => {}
			};
		}
		Err(err) => anyhow::bail!("{err:?}"),
	}
	Ok(())
}
