#![allow(unused)]
mod block;
pub use block::*;
// A block whose resource is a store, its erased half and the generic runtime
// attach; the marker naming which store block is the repo store, and the
// block naming a store this deploy does not create. Feature-free, so a lean
// binary reads the same declarations the deployer does.
mod store_block;
pub use store_block::*;
mod repo_store_block;
pub use repo_store_block::*;
mod store_uri_block;
pub use store_uri_block::*;
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
// The AWS IAM lowering every AWS compute block shares. Gated on the bindings
// feature the s3 block (and so every declared grant kind) needs.
#[cfg(feature = "bindings_aws_common")]
mod iam_policy;
#[cfg(feature = "bindings_aws_common")]
pub use iam_policy::*;
// The DNS module is reused by the lambda, fargate and lightsail blocks, and by
// the cloudflare failover (which calls its `ensure_cloudflare_provider`); gate
// it on any of them.
#[cfg(any(
	feature = "lambda_block",
	feature = "fargate_block",
	feature = "lightsail_block",
	feature = "cloudflare_dns"
))]
mod dns;
#[cfg(any(
	feature = "lambda_block",
	feature = "fargate_block",
	feature = "lightsail_block",
	feature = "cloudflare_dns"
))]
pub use dns::*;
// The opt-in Cloudflare LB failover (uses the dns module + cloudflare LB bindings).
#[cfg(feature = "cloudflare_dns")]
mod failover;
#[cfg(feature = "cloudflare_dns")]
pub use failover::*;
#[cfg(feature = "bindings_aws_common")]
mod s3_bucket_block;
#[cfg(feature = "bindings_aws_common")]
pub use s3_bucket_block::*;
// The network the AWS compute blocks sit in, and the company database inside
// it. Neither is mail-specific: the mail stack is simply their first consumer.
#[cfg(feature = "vpc_block")]
mod vpc_block;
#[cfg(feature = "vpc_block")]
pub use vpc_block::*;
#[cfg(feature = "rds_postgres_block")]
mod rds_postgres_block;
#[cfg(feature = "rds_postgres_block")]
pub use rds_postgres_block::*;
#[cfg(feature = "bindings_aws_dynamo")]
mod dynamo_table_block;
#[cfg(feature = "bindings_aws_dynamo")]
pub use dynamo_table_block::*;
// The recurring timer: an EventBridge schedule invoking a `LambdaBlock`, so its
// feature pulls the block whose function ident it composes.
#[cfg(feature = "scheduled_job_block")]
mod scheduled_job_block;
#[cfg(feature = "scheduled_job_block")]
pub use scheduled_job_block::*;
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
