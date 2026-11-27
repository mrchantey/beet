use super::*;
use beet_core::prelude::*;

/// The request's path below its router's root, refusing one that climbs out
/// of it, see [`Url::fwd_path`].
impl FromRequestMeta<Self> for FwdPath {
	fn from_request_meta(req: &RequestMeta) -> Result<Self> {
		req.url().fwd_path()
	}
}

impl From<FwdPath> for Request {
	fn from(value: FwdPath) -> Request { Request::new(HttpMethod::Get, value) }
}

#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	fn reads_the_request_path() {
		let parts = RequestParts::get("/api/users/123");
		parts.url().fwd_path().unwrap().xpect_eq(fwd_path!("api/users/123"));
		RequestParts::get("/api/../../secret")
			.url()
			.fwd_path()
			.unwrap_err();
	}
}
