use crate::prelude::*;
use anyhow::Result;




impl Relay {
	pub fn add_responder<Req: Payload, Res: Payload>(
		&self,
		address: impl Into<TopicAddress>,
		method: TopicMethod,
	) -> Result<Responder<Req, Res>> {
		let address = address.into();
		let topic_req =
			Topic::new(address.clone(), TopicScheme::Request, method);
		let topic_res =
			Topic::new(address.clone(), TopicScheme::Response, method);

		let req = self.add_subscriber_with_topic::<Req>(topic_req)?.recast();
		let res = self.add_publisher_with_topic::<Res>(topic_res)?.recast();
		Ok(Responder::new(req, res))
	}

	pub fn add_requester<Req: Payload, Res: Payload>(
		&self,
		address: impl Into<TopicAddress>,
		method: TopicMethod,
	) -> Result<Requester<Req, Res>> {
		let address = address.into();
		let topic_req =
			Topic::new(address.clone(), TopicScheme::Request, method);
		let topic_res =
			Topic::new(address.clone(), TopicScheme::Response, method);

		let req = self.add_publisher_with_topic::<Req>(topic_req)?.recast();
		let res = self.add_subscriber_with_topic::<Res>(topic_res)?.recast();
		Ok(Requester::new(req, res))
	}
}
