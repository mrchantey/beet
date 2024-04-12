// use beet::prelude::*;
// use bevy::prelude::*;

// pub fn spawn_vehicle(mut commands: Commands, messages: Res<Inbox>) {
// 	for msg in messages.iter() {
// 		match msg {
// 			Message::Behavior(BehaviorMessage::SpawnGraphVehicle(
// 				SpawnGraphVehicleMessage { graph },
// 			)) => {
// 				let target = commands
// 					.spawn((
// 						DualMotorValue::splat(MotorValue::stop()),
// 						DepthSensor::new(Vec3::default()),
// 					))
// 					.id();

// 				let entities = graph.spawn(&mut commands, target);
// 				commands
// 					.entity(target)
// 					.insert(VehicleInstance::new(entities));
// 			}
// 			_ => {}
// 		}
// 	}
// }
