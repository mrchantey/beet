//! Sending a request from inside a provider whose contract hands back a
//! [`SendBoxedFuture`], ie an http store or a PDS over xrpc.
//!
//! The one cfg ladder for it: the browser's fetch future is not `Send`, a
//! native transport's is, and with no transport compiled the provider still
//! exists (a scene declares it) but cannot reach the network.
use crate::prelude::*;
use beet_core::prelude::*;

cfg_if! {
	if #[cfg(target_arch = "wasm32")] {
		/// Box a fetch for the provider contract: the browser's fetch future is
		/// not `Send`, and the provider is only ever polled on the one thread.
		pub(crate) fn boxed<T>(
			fut: impl 'static + Future<Output = T>,
		) -> SendBoxedFuture<T> {
			Box::pin(send_wrapper::SendWrapper::new(fut))
		}

		/// The fetch api.
		pub(crate) async fn send(request: Request) -> Result<Response> {
			request.send().await
		}
	} else if #[cfg(any(feature = "ureq", feature = "reqwest"))] {
		/// Box a fetch for the provider contract.
		pub(crate) fn boxed<T>(
			fut: impl 'static + Send + Future<Output = T>,
		) -> SendBoxedFuture<T> {
			Box::pin(fut)
		}

		/// The compiled native transport, whose future is `Send`.
		pub(crate) async fn send(request: Request) -> Result<Response> {
			request.send().await
		}
	} else {
		/// Box a fetch for the provider contract.
		pub(crate) fn boxed<T>(
			fut: impl 'static + Send + Future<Output = T>,
		) -> SendBoxedFuture<T> {
			Box::pin(fut)
		}

		/// No transport compiled in: not routed through [`Request::send`],
		/// whose runtime-installed fallback is a no_std hook with no `Send`
		/// future to give a provider.
		pub(crate) async fn send(request: Request) -> Result<Response> {
			bevybail!(
				"cannot fetch `{}`: this build has no http transport (enable \
				 `ureq` or `reqwest`)",
				request.url()
			)
		}
	}
}
