use crate::prelude::*;
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
			self.handle_next(&mut handler).await?;
		}
	}

	pub async fn handle_next(
		&mut self,
		mut handler: impl FnMut(Req) -> Res,
	) -> Result<()> {
		let recv = self.req.recv_inner_mut();
		if let Ok(next) = recv.recv_direct_overflow_ok().await {
			let id = next.id;
			let response = handler(next.payload()?);
			self.res.channel_inner().push(StateMessage::new(
				self.res.topic().clone(),
				&response,
				id,
			)?)?;
		}
		Ok(())
	}

	pub fn try_handle_next(
		&mut self,
		mut handler: impl FnMut(Req) -> Res,
	) -> Result<()> {
		let recv = self.req.recv_inner_mut();
		if let Ok(next) = recv.try_recv_overflow_ok() {
			let id = next.id;
			let response = handler(next.payload()?);
			self.res.channel_inner().push(StateMessage::new(
				self.res.topic().clone(),
				&response,
				id,
			)?)?;
		}
		Ok(())
	}

	pub fn req(&self) -> &Subscriber<Req> { &self.req }
	pub fn res(&self) -> &Publisher<Res> { &self.res }
	pub fn req_mut(&mut self) -> &mut Subscriber<Req> { &mut self.req }
	pub fn res_mut(&mut self) -> &mut Publisher<Res> { &mut self.res }
}
