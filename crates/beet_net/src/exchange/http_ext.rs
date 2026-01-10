//! HTTP parsing utilities.
//!
//! This module provides utility functions for parsing and working with
//! the `http` crate types. All intense parsing of HTTP types should occur here.

/// Check if HTTP request parts indicate a body is present based on headers.
pub fn has_body(parts: &http::request::Parts) -> bool {
	has_body_by_content_length(&parts.headers)
		|| has_body_by_transfer_encoding(&parts.headers)
}

/// Check if headers indicate a body by content-length > 0.
pub fn has_body_by_content_length(headers: &http::HeaderMap) -> bool {
	headers
		.get("content-length")
		.and_then(|val| val.to_str().ok())
		.and_then(|str| str.parse::<usize>().ok())
		.map(|len| len > 0)
		.unwrap_or(false)
}

/// Check if headers indicate a body by chunked transfer encoding.
pub fn has_body_by_transfer_encoding(headers: &http::HeaderMap) -> bool {
	headers
		.get("transfer-encoding")
		.and_then(|val| val.to_str().ok())
		.map(|str| str.contains("chunked"))
		.unwrap_or(false)
}

/// Convert http version to string representation.
pub fn version_to_string(version: http::Version) -> String {
	match version {
		http::Version::HTTP_09 => "0.9".to_string(),
		http::Version::HTTP_10 => "1.0".to_string(),
		http::Version::HTTP_11 => "1.1".to_string(),
		http::Version::HTTP_2 => "2".to_string(),
		http::Version::HTTP_3 => "3".to_string(),
		_ => "1.1".to_string(),
	}
}

/// Parse a version string into an http::Version.
pub fn parse_version(version: &str) -> http::Version {
	match version {
		"0.9" => http::Version::HTTP_09,
		"1.0" => http::Version::HTTP_10,
		"1.1" => http::Version::HTTP_11,
		"2" | "2.0" => http::Version::HTTP_2,
		"3" | "3.0" => http::Version::HTTP_3,
		_ => http::Version::HTTP_11,
	}
}

/// The default HTTP version string.
pub const DEFAULT_HTTP_VERSION: &str = "1.1";

/// The default CLI version string.
pub const DEFAULT_CLI_VERSION: &str = "0.1.0";

#[cfg(test)]
mod test {
	use super::*;
	use beet_core::prelude::*;

	#[test]
	fn has_body_with_content_length() {
		let parts = http::Request::builder()
			.method(http::Method::POST)
			.uri("/test")
			.header("content-length", "5")
			.body(())
			.unwrap()
			.into_parts()
			.0;

		has_body(&parts).xpect_true();
	}

	#[test]
	fn has_body_without_headers() {
		let parts = http::Request::builder()
			.method(http::Method::GET)
			.uri("/test")
			.body(())
			.unwrap()
			.into_parts()
			.0;

		has_body(&parts).xpect_false();
	}

	#[test]
	fn has_body_with_chunked_encoding() {
		let parts = http::Request::builder()
			.method(http::Method::POST)
			.uri("/test")
			.header("transfer-encoding", "chunked")
			.body(())
			.unwrap()
			.into_parts()
			.0;

		has_body(&parts).xpect_true();
	}

	#[test]
	fn version_conversions() {
		version_to_string(http::Version::HTTP_11).xpect_eq("1.1");
		version_to_string(http::Version::HTTP_2).xpect_eq("2");
		parse_version("1.1").xpect_eq(http::Version::HTTP_11);
		parse_version("2").xpect_eq(http::Version::HTTP_2);
	}
}
