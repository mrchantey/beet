use async_channel::Sender;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Added to each exchange entity, storing the [`Sender<Response>`]
/// for when it is complete.
#[derive(Debug, Component)]
pub struct ExchangeContext {
	/// A channel crated by the `flow_route_handler`,
	/// sending this notifies the observer on the [`Router`]
	sender: Sender<Response>,
}

impl ExchangeContext {
	pub fn new(sender: Sender<Response>) -> Self { Self { sender } }
	pub(super) fn sender(&self) -> &Sender<Response> { &self.sender }
}
