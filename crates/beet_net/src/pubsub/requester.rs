use super::*;
use crate::utils::ReceiverTExt;
use anyhow::Result;
use bevy_utils::Duration;
use std::ops::ControlFlow;

#[derive(Debug, Clone)]
pub struct Requester<Req: Payload, Res: Payload> {
	pub(crate) req: Publisher<Req>,
	pub(crate) res: Subscriber<Res>,
}

impl<Req: Payload, Res: Payload> Requester<Req, Res> {
	pub fn new(req: Publisher<Req>, res: Subscriber<Res>) -> Self {
		Self { req, res }
	}

	pub fn start_request(&mut self, req: &Req) -> Result<MessageId> {
		self.req.push(&req)
	}

	pub fn block_on_response(&mut self, id: MessageId) -> Result<Res> {
		let recv = self.res.recv_inner_mut();
		loop {
			match Self::check_received(recv.try_recv_overflow_ok()?, id)? {
				ControlFlow::Break(val) => break Ok(val),
				_ => {}
			}
			std::thread::sleep(Duration::from_millis(16));
		}
	}

	pub async fn request(&mut self, req: &Req) -> Result<Res> {
		let id = self.req.push(req)?;
		let recv = self.res.recv_inner_mut();
		loop {
			match Self::check_received(
				recv.recv_direct_overflow_ok().await?,
				id,
			)? {
				ControlFlow::Break(val) => break Ok(val),
				_ => {}
			}
		}
	}

	fn check_received(
		msg: StateMessage,
		id: MessageId,
	) -> Result<ControlFlow<Res, ()>> {
		if msg.id == id {
			Ok(ControlFlow::Break(msg.payload()?))
		} else {
			Ok(ControlFlow::Continue(()))
		}
	}

	pub fn req(&self) -> &Publisher<Req> { &self.req }
	pub fn res(&self) -> &Subscriber<Res> { &self.res }
	pub fn req_mut(&mut self) -> &mut Publisher<Req> { &mut self.req }
	pub fn res_mut(&mut self) -> &mut Subscriber<Res> { &mut self.res }
}
