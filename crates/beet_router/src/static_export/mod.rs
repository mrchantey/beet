//! Static-site generation.
//!
//! Renders the router's static routes to HTML and writes them to a
//! [`BlobStore`](beet_net::prelude::BlobStore), ready to deploy.

mod export_static;

pub use export_static::*;
