use crate::prelude::*;
use beet_core::prelude::*;

pub(super) async fn send_ureq(req: Request) -> Result<Response> {
	super::send::check_https_features(&req)?;

	let (parts, body) = req.into_parts();

	// Build the agent with proper TLS configuration

	#[cfg(all(feature = "native-tls", not(feature = "rustls-tls")))]
	let agent = ureq::config::Config::builder()
		.tls_config(
			ureq::tls::TlsConfig::builder()
				.provider(ureq::tls::TlsProvider::NativeTls)
				.build(),
		)
		.build()
		.new_agent();
	#[cfg(all(feature = "rustls-tls", not(feature = "native-tls")))]
	let agent = ureq::config::Config::builder()
		.tls_config(
			ureq::tls::TlsConfig::builder()
				.provider(ureq::tls::TlsProvider::NativeTls)
				.build(),
		)
		.build()
		.new_agent();
	#[cfg(not(any(feature = "rustls-tls", feature = "native-tls")))]
	let agent = ureq::config::Config::builder().build().new_agent();

	// Convert to http::Request
	let http_parts: http::request::Parts = parts.try_into()?;
	let body = body.into_bytes().await?.to_vec();
	let http_req = http::Request::from_parts(http_parts, body);

	let res = agent.run(http_req).map_err(BevyError::from)?;
	let res: Response = into_response(res)?;
	Ok(res)
}

fn into_response(res: http::Response<ureq::Body>) -> Result<Response> {
	let status = res.status();

	// Build ResponseParts with headers
	let parts = {
		let mut builder = PartsBuilder::new();
		for (key, value) in res.headers().iter() {
			if let Ok(value_str) = value.to_str() {
				builder = builder.header(key.to_string(), value_str);
			}
		}
		builder.build_response_parts(status)
	};

	// ureq is synchronous, so just read the whole body into bytes
	let bytes_vec = res.into_body().read_to_vec().map_err(BevyError::from)?;
	let body = Body::Bytes(bytes::Bytes::from(bytes_vec));

	Ok(Response::from_parts(parts, bytes::Bytes::new()).with_body(body))
}
