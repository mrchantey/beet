// use crate::prelude::*;
// use anyhow::Result;
// use beet::prelude::*;
// use bevy::prelude::Vec3;
// use esp_idf_hal::delay::FreeRtos;
// use forky_core::ResultTEExt;
// use std::sync::Arc;
// use std::sync::Mutex;

// pub async fn start_remote_client() -> Result<!> {
// 	let AppHardware {
// 		hbridge,
// 		mut ultrasound,
// 		modem,
// 		sys_loop,
// 	} = AppHardware::new()?;

// 	let mut wifi = WifiClient::new(modem, sys_loop)?;
// 	wifi.connect().await?;
// 	let mut ws = EspWsClient::new()?;
// 	ws.await_upgrade().await?;
// 	let hbridge = Arc::new(Mutex::new(hbridge));

// 	ws.add_listener(move |msg: &Message| {
// 		match msg {
// 			Message::Behavior(BehaviorMessage::SetMotors(motors)) => {
// 				log::info!("<<< {motors:?}");
// 				hbridge
// 					.lock()
// 					.unwrap()
// 					.from_dual_motor_value(&motors)
// 					.ok_or(|e| log::error!("{e}"));
// 			}
// 			_other => {
// 				// log::info!("unhandled message: {_other:?}")
// 			}
// 		}
// 		// Ok(())
// 	});


// 	let mut sensor = DepthSensor::new(Vec3::new(0., 0.02, 0.));
// 	let epsilon = 0.05;


// 	let delay = 10;
// 	let smoothness = 5;
// 	let buffer_size = delay * smoothness;
// 	let mut smoother = Smoother::new(buffer_size);

// 	loop {
// 		let depth = ultrasound.measure_or_max();
// 		let depth = smoother.add_and_smooth(depth);
// 		if (sensor.value - depth).abs() > epsilon {
// 			log::info!(">>> depth {depth:.2}");
// 			sensor.value = depth;
// 			ws.send(Message::SendTo(SendTo::Channel(vec![
// 				BehaviorMessage::SetDepth(sensor.clone()).into(),
// 			])))
// 			.await?;
// 		}
// 		FreeRtos::delay_ms(delay as u32);
// 	}
// }
