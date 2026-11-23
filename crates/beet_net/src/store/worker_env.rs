//! The Cloudflare Worker runtime as the stores see it: the ambient
//! [`worker::Env`] each invocation carries, and the request and response
//! conversions between the runtime's types and beet's.
use crate::prelude::*;
use beet_core::prelude::*;
use std::cell::RefCell;

thread_local! {
	/// The ambient Worker [`Env`](worker::Env), set at the top of each
	/// handler so a binding resolves for the invocation. The runtime is
	/// single-threaded, so a thread-local is the isolate-global slot.
	static WORKER_ENV: RefCell<Option<worker::Env>> = const { RefCell::new(None) };
}

/// The ambient Worker runtime: the [`worker::Env`] an invocation carries, so
/// a store declared by binding name ([`R2WorkersStore`],
/// [`DurableObjectStore`]) resolves its live binding, and the conversions a
/// handler crosses at its edges.
pub struct WorkerEnv;

impl WorkerEnv {
	/// Stash the invocation's [`worker::Env`] for every binding resolved
	/// during it. Call at the top of each handler, before building or driving
	/// the app.
	pub fn set(env: worker::Env) {
		WORKER_ENV.with(|slot| *slot.borrow_mut() = Some(env));
	}

	/// Run `func` with the ambient env, refused when no handler set one.
	pub fn with<T>(func: impl FnOnce(&worker::Env) -> Result<T>) -> Result<T> {
		WORKER_ENV.with(|slot| match slot.borrow().as_ref() {
			Some(env) => func(env),
			None => bevybail!(
				"no `worker::Env` set: call `WorkerEnv::set` at the top of the \
				 Worker's handler"
			),
		})
	}

	/// A [`worker::Request`] as a beet [`Request`]: method, full url,
	/// headers and the body bytes.
	pub async fn to_request(mut req: worker::Request) -> Result<Request> {
		let url = req.url()?;
		// `worker::Method` displays as its uppercase name, which `HttpMethod` parses
		let method = req.method().to_string().parse::<HttpMethod>()?;
		// the body is read up front, a Worker request being consumed once
		let body = req.bytes().await.unwrap_or_default();
		let mut parts = RequestParts::new(method, Url::coerce(url.as_str()));
		for (key, value) in req.headers() {
			parts.headers.set_raw(key, value);
		}
		let body = match body.is_empty() {
			true => Body::default(),
			false => Body::Bytes(body.into()),
		};
		Request::from_parts(parts, body).xok()
	}

	/// A beet [`Request`] as a [`worker::Request`] to `url`, ie a Durable
	/// Object's stub fetch.
	pub async fn to_worker_request(
		request: Request,
		url: &str,
	) -> Result<worker::Request> {
		let (parts, body) = request.into_parts();
		let body = body.into_bytes().await?;
		let headers = worker::Headers::new();
		for (key, values) in parts.headers.iter_all() {
			for value in values {
				headers.append(key, value)?;
			}
		}
		let mut init = worker::RequestInit::new();
		init.with_method(worker::Method::from(
			parts.method().to_string().to_uppercase(),
		))
		.with_headers(headers);
		if !body.is_empty() {
			init.with_body(Some(
				js_sys::Uint8Array::from(body.as_ref()).into(),
			));
		}
		worker::Request::new_with_init(url, &init)?.xok()
	}

	/// A beet [`Response`] as a [`worker::Response`]: the body bytes, the
	/// status and the headers.
	pub async fn to_worker_response(
		response: Response,
	) -> Result<worker::Response> {
		let (parts, body) = response.into_parts();
		let bytes = body.into_bytes().await?;
		let mut worker_response = worker::Response::from_bytes(bytes.to_vec())?
			.with_status(parts.status().as_u16());
		let headers = worker_response.headers_mut();
		for (key, values) in parts.headers.iter_all() {
			for value in values {
				headers.append(key, value)?;
			}
		}
		worker_response.xok()
	}

	/// A [`worker::Response`] as a beet [`Response`].
	pub async fn to_response(
		mut response: worker::Response,
	) -> Result<Response> {
		let status = StatusCode::new(response.status_code());
		let bytes = response.bytes().await?;
		let mut out = Response::from_status(status).with_body(bytes);
		for (key, value) in response.headers() {
			out = out.with_header(&key, &value);
		}
		out.xok()
	}
}
