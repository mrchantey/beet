use beet::prelude::*;
use embedded_svc::ws::FrameType;
use esp_idf_hal::io::EspIOError;
use esp_idf_hal::task::block_on;
use esp_idf_svc::ws::client::EspWebSocketClient;
use esp_idf_svc::ws::client::EspWebSocketClientConfig;
use esp_idf_svc::ws::client::WebSocketEvent;
use esp_idf_svc::ws::client::WebSocketEventType;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

// 4096 - too big
// 2048 - works, but i think its pretty close
const CIBORIUM_SCRATCH_BUFFER_SIZE: usize = 4096;


pub type EspWsEvent = WebSocketEvent<'static>;

pub struct EspWsClient {
	// pub ws: EspWebSocketClient<'static>,
	pub channel: WsChannel,
}

impl WsChannelHelpers for EspWsClient {
	fn channel(&self) -> &WsChannel { &self.channel }
	fn channel_mut(&mut self) -> &mut WsChannel { &mut self.channel }
}


impl EspWsClient {
	pub fn new() -> anyhow::Result<Self> {
		let timeout = Duration::from_secs(10);
		let config = EspWebSocketClientConfig {
			server_cert: None,
			reconnect_timeout_ms: timeout, //default 10s
			network_timeout_ms: timeout,   //default 10s
			task_stack: 5_000, // 10_000 - not enough heap, default - stack overflow
			..Default::default()
		};

		let url = DEFAULT_WS_LOCAL_URL;
		let timeout = Duration::from_secs(10);

		let mut recv = WsRecv::new(WsSend::dummy());
		let mut recv2 = recv.clone();
		let ws =
			EspWebSocketClient::new(&url, &config, timeout, move |event| {
				block_on(async {
					if let Err(err) = parse(event, &mut recv2).await {
						log::error!("WS Recv Error: {:?}", err);
					}
				});
			})?;

		let ws = Arc::new(Mutex::new(ws));

		let send = WsSend::new(async move |msg: Message| {
			let mut ws = ws.lock().unwrap();
			ws.send(FrameType::Binary(false), &msg.to_bytes()?)?;
			Ok(())
		});

		recv.set_send(send.clone());
		let channel = WsChannel {
			send,
			recv,
			dest_client: None,
		}
		.as_client(ConnectParams::default_esp());

		Ok(Self { channel })
	}
}

impl Drop for EspWsClient {
	fn drop(&mut self) {
		log::info!("EspWsClient Dropped");
	}
}

async fn parse(
	event: &Result<WebSocketEvent<'_>, EspIOError>,
	recv: &mut WsRecv,
) -> anyhow::Result<()> {
	match event {
		Ok(event) => {
			match event.event_type {
				WebSocketEventType::Text(_value) => {
					log::error!("Receiving text Socket Messages on ESP32 is not supported");
				}
				WebSocketEventType::Binary(value) => {
					// safer to keep on heap, but watch for framentation
					let mut scratch_buffer =
						Box::new([0; CIBORIUM_SCRATCH_BUFFER_SIZE]);

					let msg = Message::from_bytes_with_buffer(
						value,
						scratch_buffer.as_mut(),
					)?;
					recv.recv(msg).await?;
				}
				_ => {}
			};
		}
		Err(err) => anyhow::bail!("{err:?}"),
	}
	Ok(())
}
