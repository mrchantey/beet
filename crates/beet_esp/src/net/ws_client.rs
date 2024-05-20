use anyhow::Result;
use beet_net::extensions::FlumeReceiverExt;
use beet_net::networking::Transport;
use dotenv_codegen::dotenv;
use embedded_svc::ws::FrameType;
use esp_idf_hal::io::EspIOError;
use esp_idf_svc::ws::client::EspWebSocketClient;
use esp_idf_svc::ws::client::EspWebSocketClientConfig;
use esp_idf_svc::ws::client::WebSocketEvent;
use esp_idf_svc::ws::client::WebSocketEventType;
use flume::Receiver;
use flume::Sender;
use forky_core::ResultTEExt;
use std::time::Duration;

// 4096 - too big
// 2048 - works, but i think its pretty close
// const CIBORIUM_SCRATCH_BUFFER_SIZE: usize = 4096;

pub struct WsClient {
	pub ws: EspWebSocketClient<'static>,
	// pub outgoing_send: Sender<Vec<u8>>,
	// pub incoming_send: Sender<Vec<u8>>,
	incoming_recv: Receiver<Vec<u8>>,
	// outgoing_recv: Receiver<Vec<u8>>,
}

impl WsClient {
	pub fn new() -> anyhow::Result<Self> {
		let (incoming_send, incoming_recv) = flume::unbounded();
		Self::new_with_channels(incoming_send, incoming_recv)
	}

	pub fn new_with_channels(
		mut incoming_send: Sender<Vec<u8>>,
		incoming_recv: Receiver<Vec<u8>>,
		// outgoing_send: Sender<Vec<u8>>,
		// outgoing_recv: Receiver<Vec<u8>>,
	) -> anyhow::Result<Self> {
		let timeout = Duration::from_secs(10);
		let config = EspWebSocketClientConfig {
			server_cert: None,
			reconnect_timeout_ms: timeout, //default 10s
			network_timeout_ms: timeout,   //default 10s
			task_stack: 5_000, // 10_000 - not enough heap, default - stack overflow
			
			..Default::default()
		};

		let url = dotenv!("WS_URL");

		// let mut incoming_send2 = incoming_send.clone();
		let ws =
			EspWebSocketClient::new(&url, &config, timeout, move |event| {
				parse(event, &mut incoming_send).ok_or(|e| log::error!("{e}"));
			})?;

		Ok(Self { ws, incoming_recv })
	}

	pub fn is_connected(&self) -> bool { self.ws.is_connected() }
}

impl Drop for WsClient {
	fn drop(&mut self) {
		log::info!("EspWsClient Dropped");
	}
}

fn parse(
	event: &Result<WebSocketEvent<'_>, EspIOError>,
	send: &mut Sender<Vec<u8>>,
) -> anyhow::Result<()> {
	match event {
		Ok(event) => {
			match event.event_type {
				WebSocketEventType::Text(_value) => {
					log::error!("Receiving text Socket Messages on ESP32 is not supported");
				}
				WebSocketEventType::Binary(value) => {
					send.send(value.to_vec())?;
				}
				_ => {}
			};
		}
		Err(err) => anyhow::bail!("{err:?}"),
	}
	Ok(())
}


impl Transport for WsClient {
	async fn send_bytes(&mut self, bytes: Vec<u8>) -> Result<()> {
		self.ws.send(FrameType::Binary(false), &bytes)?;
		Ok(())
	}

	fn recv_bytes(&mut self) -> Result<Vec<Vec<u8>>> {
		self.incoming_recv.try_recv_all()
	}
}
