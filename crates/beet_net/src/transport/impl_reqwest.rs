use crate::prelude::*;
use beet_core::prelude::*;
use reqwest::Client;
use reqwest::RequestBuilder;
use std::sync::LazyLock;

#[allow(unused)]
pub(super) async fn send_reqwest(req: Request) -> Result<Response> {
	static REQWEST_CLIENT: LazyLock<Client> = LazyLock::new(|| Client::new());

	super::send::check_https_features(&req)?;

	let req: reqwest::Request = into_request(req)?;
	let res = RequestBuilder::from_parts(REQWEST_CLIENT.clone(), req)
		.send()
		.await?;
	let res: Response = into_response(res).await?;
	Ok(res)
}

fn into_request(request: Request) -> Result<reqwest::Request> {
	let (parts, body) = request.into_parts();
	match body {
		Body::Bytes(bytes) => {
			// Convert our parts to http parts, then to reqwest
			let http_parts: http::request::Parts = parts.try_into()?;
			let http_req = http::Request::from_parts(http_parts, bytes);
			http_req.try_into().map_err(BevyError::from)
		}
		Body::Stream(stream) => {
			use futures::TryStreamExt;
			use reqwest::Body as ReqwestBody;

			let stream_inner = stream.take();
			let reqwest_body =
				ReqwestBody::wrap_stream(stream_inner.map_err(|err| {
					std::io::Error::new(
						std::io::ErrorKind::Other,
						format!("{}", err),
					)
				}));

			// Build the method
			let method: http::Method = (*parts.method()).into();

			// Build the URL from the URI
			let url: reqwest::Url =
				parts.uri().parse().map_err(BevyError::from)?;

			let mut builder = reqwest::Request::new(method, url);

			// Convert our headers to http::HeaderMap
			let mut headers = http::HeaderMap::new();
			for (key, values) in parts.headers().iter_all() {
				if let Ok(header_name) =
					http::header::HeaderName::from_bytes(key.as_bytes())
				{
					for value in values {
						if let Ok(header_value) =
							http::header::HeaderValue::from_str(value)
						{
							headers.append(header_name.clone(), header_value);
						}
					}
				}
			}

			*builder.headers_mut() = headers;
			*builder.body_mut() = Some(reqwest_body);

			Ok(builder)
		}
	}
}


async fn into_response(res: reqwest::Response) -> Result<Response> {
	let status = res.status();

	// Copy headers to our ResponseParts using PartsBuilder
	let parts = {
		let mut builder = PartsBuilder::new();
		for (key, value) in res.headers().iter() {
			if let Ok(value_str) = value.to_str() {
				builder = builder.header(key.to_string(), value_str);
			}
		}
		builder.build_response_parts(status)
	};

	let is_bytes = res
		.headers()
		.get("content-length")
		.and_then(|val| val.to_str().ok())
		.and_then(|str| str.parse::<u64>().ok())
		.map_or(false, |val| val <= Body::MAX_BUFFER_SIZE as u64);

	let body = if is_bytes {
		Body::Bytes(res.bytes().await?.into())
	} else {
		use futures::TryStreamExt;
		use send_wrapper::SendWrapper;

		Body::Stream(SendWrapper::new(Box::pin(
			res.bytes_stream().map_err(BevyError::from),
		)))
	};

	Ok(Response::from_parts(parts, bytes::Bytes::new()).with_body(body))
}
