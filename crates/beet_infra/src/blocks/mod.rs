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
// The DNS module is reused by the lambda and fargate blocks; gate it on either.
#[cfg(any(feature = "lambda_block", feature = "fargate_block"))]
mod dns;
#[cfg(any(feature = "lambda_block", feature = "fargate_block"))]
pub use dns::*;
// The opt-in Cloudflare LB failover (uses the dns module + cloudflare LB bindings).
#[cfg(any(feature = "lambda_block", feature = "fargate_block"))]
mod failover;
#[cfg(any(feature = "lambda_block", feature = "fargate_block"))]
pub use failover::*;
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
