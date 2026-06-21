#![allow(unused)]
#[cfg(feature = "lambda_block")]
mod lambda;
#[cfg(feature = "lambda_block")]
pub use lambda::*;
#[cfg(feature = "lightsail_block")]
mod lightsail;
#[cfg(feature = "lightsail_block")]
pub use lightsail::*;
#[cfg(feature = "fargate_block")]
mod fargate;
#[cfg(feature = "fargate_block")]
pub use fargate::*;
mod block;
pub use block::*;
#[cfg(feature = "bindings_aws_common")]
mod s3_bucket_block;
#[cfg(feature = "bindings_aws_common")]
pub use s3_bucket_block::*;
// Cloudflare blocks are plain config components (wrangler-provisioned, not
// terraform), so they need no bindings feature.
#[cfg(feature = "cloudflare_block")]
mod cloudflare_container;
#[cfg(feature = "cloudflare_block")]
pub use cloudflare_container::*;
#[cfg(feature = "cloudflare_block")]
mod cloudflare_worker;
#[cfg(feature = "cloudflare_block")]
pub use cloudflare_worker::*;
