use super::*;
use anyhow::Result;



#[derive(Debug, Clone)]
pub struct Responder<Req: Payload, Res: Payload> {
	pub(crate) req: Subscriber<Req>,
	pub(crate) res: Publisher<Res>,
}

impl<Req: Payload, Res: Payload> Responder<Req, Res> {
	pub fn new(req: Subscriber<Req>, res: Publisher<Res>) -> Self {
		Self { req, res }
	}
	pub async fn handle_requests_forever(
		&mut self,
		mut handler: impl FnMut(Req) -> Res,
	) -> Result<!> {
		loop {
			self.handle_request(&mut handler).await?;
		}
	}

	pub async fn handle_request(
		&mut self,
		mut handler: impl FnMut(Req) -> Res,
	) -> Result<()> {
		let recv = self.req.recv_inner_mut();
		if let Ok(next) = recv.recv_direct().await {
			let id = next.id;
			let response = handler(next.payload()?);
			self.res
				.send_inner()
				.broadcast_direct(StateMessage::new(
					self.res.topic().clone(),
					&response,
					id,
				)?)
				.await?;
		}
		Ok(())
	}
	pub fn req(&self) -> &Subscriber<Req> { &self.req }
	pub fn res(&self) -> &Publisher<Res> { &self.res }
	pub fn req_mut(&mut self) -> &mut Subscriber<Req> { &mut self.req }
	pub fn res_mut(&mut self) -> &mut Publisher<Res> { &mut self.res }
}
