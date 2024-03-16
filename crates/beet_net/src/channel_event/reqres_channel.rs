use crate::utils::flumeReceiverTExt;
use anyhow::Result;
use flume::Receiver;
use flume::Sender;

pub fn reqres_channel<Req, Res>(
) -> (RequestChannel<Req, Res>, ResponseChannel<Req, Res>) {
	let (req_send, req_recv) = flume::unbounded();
	let (res_send, res_recv) = flume::unbounded();
	(
		RequestChannel::new(req_send, res_recv),
		ResponseChannel::new(req_recv, res_send),
	)
}

pub struct RequestChannel<Req, Res> {
	pub req: Sender<Req>,
	pub res: Receiver<Res>,
}

impl<Req, Res> RequestChannel<Req, Res> {
	pub fn new(req: Sender<Req>, res: Receiver<Res>) -> Self {
		Self { req, res }
	}
	pub fn request(&self, req: Req) -> Result<Res> {
		self.req.send(req).map_err(|e| anyhow::anyhow!("{}", e))?;
		let res = self.res.recv()?;
		Ok(res)
	}
	pub fn start_request(&self, req: Req) -> Result<()> {
		self.req.send(req).map_err(|e| anyhow::anyhow!("{}", e))?;
		Ok(())
	}
	pub fn block_on_response(&self) -> Result<Res> {
		let res = self.res.recv()?;
		Ok(res)
	}

	pub async fn request_async(&self, req: Req) -> Result<Res> {
		self.req
			.send_async(req)
			.await
			.map_err(|e| anyhow::anyhow!("{}", e))?;
		let res = self.res.recv_async().await?;
		Ok(res)
	}
}

pub struct ResponseChannel<Req, Res> {
	pub req: Receiver<Req>,
	pub res: Sender<Res>,
}

impl<Req, Res> ResponseChannel<Req, Res> {
	pub fn new(req: Receiver<Req>, res: Sender<Res>) -> Self {
		Self { req, res }
	}

	pub fn try_respond(
		&mut self,
		mut handler: impl FnMut(Req) -> Res,
	) -> Result<()> {
		for req in self.req.try_recv_all()? {
			let res = handler(req);
			self.res.send(res).map_err(|e| anyhow::anyhow!("{}", e))?;
		}
		Ok(())
	}
}

