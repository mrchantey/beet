use super::*;
use crate::prelude::*;
use anyhow::Result;




#[derive(Debug, Clone)]
pub struct Requester<Req: Payload, Res: Payload> {
	pub(crate) req: Publisher<Req>,
	pub(crate) res: Subscriber<Res>,
}

impl<Req: Payload, Res: Payload> Requester<Req, Res> {
	pub fn new(req: Publisher<Req>, res: Subscriber<Res>) -> Self {
		Self { req, res }
	}


	pub async fn request(&mut self, req: &Req) -> Result<Res> {
		let id = self.req.broadcast(req).await?;
		let recv = self.res.recv_inner_mut();
		loop {
			match recv.recv_direct().await {
				Ok(msg) => {
					if msg.id == id {
						break Ok(msg.payload()?);
					}
				}
				Err(e) => break Err(e.into()),
			}
		}
	}
	pub fn req(&self) -> &Publisher<Req> { &self.req }
	pub fn res(&self) -> &Subscriber<Res> { &self.res }
	pub fn req_mut(&mut self) -> &mut Publisher<Req> { &mut self.req }
	pub fn res_mut(&mut self) -> &mut Subscriber<Res> { &mut self.res }
}
