
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct MessageReplicon {
// 	pub channel_id: u8,
// 	pub data: Vec<u8>,
// }

// impl MessageReplicon {
// 	pub fn bytes_to_client(
// 		replicon_client: &mut RepliconClient,
// 		bytes: &[u8],
// 	) -> Result<(), bincode::Error> {
// 		bincode::deserialize::<Vec<MessageReplicon>>(bytes)?
// 			.into_iter()
// 			.for_each(|message| {
// 				replicon_client.send(message.channel_id, message.data);
// 			});
// 		Ok(())
// 	}

// 	pub fn bytes_from_client(
// 		replicon_client: &mut RepliconClient,
// 	) -> Result<Vec<u8>, bincode::Error> {
// 		let messages = replicon_client
// 			.drain_sent()
// 			.map(|(channel_id, data)| MessageReplicon {
// 				channel_id,
// 				data: data.to_vec(),
// 			})
// 			.collect::<Vec<_>>();
// 		bincode::serialize(&messages)
// 	}
// }
