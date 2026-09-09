//! HTTP interop for the core [`Url`] type: the header-map conversions and the
//! `http` crate's scheme, which cannot live in `beet_core` (it has no `http`
//! dependency) and cannot be a `From` impl here (both types would be foreign).
//!
//! The [`Url`] itself lives in [`beet_core::path`], beside [`SmolPath`]: it is
//! a logical path type, not an HTTP one, and every layer above the transport
//! speaks it.

use beet_core::prelude::*;

/// The core [`Scheme`] an `http` uri names, `Scheme::None` when it names none.
pub(crate) fn scheme_from_http(scheme: Option<&http::uri::Scheme>) -> Scheme {
	Scheme::from_str(scheme.map(|scheme| scheme.as_str()).unwrap_or_default())
}

/// Convert an [`http::HeaderMap`] to a [`super::HeaderMap`],
/// with all keys normalized to kebab-case.
pub(crate) fn http_header_map_to_header_map(
	map: &http::HeaderMap,
) -> super::HeaderMap {
	let mut header_map = super::HeaderMap::new();
	for (key, value) in map.iter() {
		let value = value.to_str().unwrap_or("<opaque-bytes>").to_string();
		header_map.set_raw(key.as_str(), value);
	}
	header_map
}

/// Convert a [`super::HeaderMap`] back to [`http::HeaderMap`].
pub(crate) fn header_map_to_http(
	headers: &super::HeaderMap,
) -> Result<http::HeaderMap, http::header::InvalidHeaderValue> {
	use core::str::FromStr;
	let mut http_headers = http::HeaderMap::new();
	for (key, values) in headers.iter_all() {
		let header_name = http::header::HeaderName::from_str(key)
			.unwrap_or_else(|_| {
				http::header::HeaderName::from_static("x-invalid")
			});
		for value in values {
			http_headers.append(
				header_name.clone(),
				http::header::HeaderValue::from_str(value)?,
			);
		}
	}
	Ok(http_headers)
}
