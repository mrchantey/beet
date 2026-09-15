//! non-generated additions to the cloudflare bindings
#[allow(unused)]
use crate::bindings::*;
#[allow(unused)]
use crate::prelude::*;

/// An R2 bucket is named by the composed resource name, exactly as an S3
/// bucket is: one label, one name, on either side of the vendor line.
#[cfg(feature = "bindings_cloudflare_common")]
impl terra::PrimaryResource for CloudflareR2BucketDetails {
	fn set_primary_identifier(&mut self, name: &str) { self.name = name.into() }
}
