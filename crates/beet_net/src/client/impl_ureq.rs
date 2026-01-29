use beet_core::prelude::*;
use bytes::Bytes;
use send_wrapper::SendWrapper;
use std::io::Read;

pub(super) async fn send_ureq(req: Request) -> Result<Response> {
	super::send::check_https_features(&req)?;

	let (parts, body) = req.into_parts();

	// Build the agent with proper TLS configuration
	// Set http_status_as_error to false so 4xx/5xx responses are not treated as errors.
	// We want to capture the actual response (headers, body, etc) regardless of status code.
	// Only IO/connection errors should fail the request.

	#[cfg(all(feature = "native-tls", not(feature = "rustls-tls")))]
	let agent = ureq::config::Config::builder()
		.tls_config(
			ureq::tls::TlsConfig::builder()
				.provider(ureq::tls::TlsProvider::NativeTls)
				.build(),
		)
		.http_status_as_error(false)
		.build()
		.new_agent();
	#[cfg(all(feature = "rustls-tls", not(feature = "native-tls")))]
	let agent = ureq::config::Config::builder()
		.tls_config(
			ureq::tls::TlsConfig::builder()
				.provider(ureq::tls::TlsProvider::NativeTls)
				.build(),
		)
		.http_status_as_error(false)
		.build()
		.new_agent();
	#[cfg(not(any(feature = "rustls-tls", feature = "native-tls")))]
	let agent = ureq::config::Config::builder()
		.http_status_as_error(false)
		.build()
		.new_agent();

	// Convert to http::Request
	let http_parts: http::request::Parts = parts.try_into()?;
	let body = body.into_bytes().await?.to_vec();
	let http_req = http::Request::from_parts(http_parts, body);

	// Run the blocking request on a thread pool to avoid blocking the async executor
	let res = blocking::unblock(move || agent.run(http_req))
		.await
		.map_err(BevyError::from)?;
	let res: Response = into_response(res)?;
	Ok(res)
}

fn into_response(res: http::Response<ureq::Body>) -> Result<Response> {
	// Check if this is a streaming response (SSE or chunked)
	let is_event_stream = res
		.headers()
		.get("content-type")
		.and_then(|v| v.to_str().ok())
		.map_or(false, |ct| ct.contains("text/event-stream"));

	let is_chunked = res
		.headers()
		.get("transfer-encoding")
		.and_then(|v| v.to_str().ok())
		.map_or(false, |te| te.contains("chunked"));

	let should_stream = is_event_stream || is_chunked;

	// Build ResponseParts with headers
	let parts = {
		let mut builder = PartsBuilder::new();
		for (key, value) in res.headers().iter() {
			if let Ok(value_str) = value.to_str() {
				builder = builder.header(key.to_string(), value_str);
			}
		}
		builder.build_response_parts(res.status().into())
	};

	let body = if should_stream {
		// Create a streaming body for SSE/chunked responses
		create_streaming_body(res.into_body())
	} else {
		// Read the whole body into bytes for regular responses
		let bytes_vec =
			res.into_body().read_to_vec().map_err(BevyError::from)?;
		Body::Bytes(Bytes::from(bytes_vec))
	};

	Ok(Response::from_parts(parts, Bytes::new()).with_body(body))
}

/// Creates a streaming body from a ureq body reader.
/// Spawns blocking reads on a thread pool and sends chunks through a channel.
fn create_streaming_body(ureq_body: ureq::Body) -> Body {
	use futures::stream;

	let (sender, receiver) = async_channel::bounded::<Result<Bytes>>(16);

	// Spawn the blocking reader on a thread pool
	blocking::unblock(move || {
		let mut reader = ureq_body.into_reader();
		let mut buf = vec![0u8; 8192];

		loop {
			match reader.read(&mut buf) {
				Ok(0) => {
					// EOF reached
					break;
				}
				Ok(n) => {
					let chunk = Bytes::copy_from_slice(&buf[..n]);
					// If receiver is dropped, stop reading
					if sender.send_blocking(Ok(chunk)).is_err() {
						break;
					}
				}
				Err(err) => {
					let _ = sender.send_blocking(Err(BevyError::from(err)));
					break;
				}
			}
		}
	})
	.detach();

	// Convert the receiver into a stream
	let byte_stream = stream::unfold(receiver, |rx| async move {
		match rx.recv().await {
			Ok(result) => Some((result, rx)),
			Err(_) => None, // Channel closed
		}
	});

	Body::Stream(SendWrapper::new(Box::pin(byte_stream)))
}
