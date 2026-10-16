//! ECS types for interacting with terraform configurations.
mod access_grant;
mod artifacts;
// what a repo's AWS deploy credential is allowed to do, lowered from the
// provider types its stacks render. Gated with the AWS IAM lowering it mirrors
// (`IamPolicy`), since both speak AWS policy documents.
#[cfg(feature = "bindings_aws_common")]
mod aws_deploy_policy;
mod build_artifact;
// what a Cloudflare action asks of the deploy token, declared on the action and
// the only way it reaches that token.
mod cloudflare_access;
// the same lowering for Cloudflare, whose api token has no policy document and
// no bindings behind it: the input is declared type names and the
// `CloudflareAccess` each action declares, so it is ungated.
mod cloudflare_deploy_token;
// the source tree a deploy was built from, recorded on its ledger.
mod deploy_source;
mod deployment;
mod infra_plugin;
// a bucket's per-prefix retention, shared by every provider's bucket block.
mod prefix_expiry;
// where a stack's resources land at each provider, resolved by ancestry.
mod provider_address;
mod resource_scope;
// the ceiling on what a deploy may create, lowered from the IAM documents the
// stacks render, so it rides the same gate as the documents themselves.
#[cfg(feature = "bindings_aws_common")]
mod runtime_boundary;
// the class a bucket keeps its objects in, read by its transition rule and by
// a push into it.
mod s3_storage_class;
// the providers a stack's secrets live in, in every build: a deployed box
// reads its secrets through them
mod secret_store;
mod stack;
mod stack_backend;
mod state_encryption;
// the cli-facing stack verbs, which drive the native tofu `Project`.
#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
mod stack_cli;
pub use access_grant::*;
pub use artifacts::*;
#[cfg(feature = "bindings_aws_common")]
pub use aws_deploy_policy::*;
pub use build_artifact::*;
pub use cloudflare_access::*;
pub use cloudflare_deploy_token::*;
pub use deploy_source::*;
pub use deployment::*;
pub use infra_plugin::*;
pub use prefix_expiry::*;
pub use provider_address::*;
pub use resource_scope::*;
#[cfg(feature = "bindings_aws_common")]
pub use runtime_boundary::*;
pub use s3_storage_class::*;
pub use secret_store::*;
pub use stack::*;
pub use stack_backend::*;
#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
pub use stack_cli::*;
pub use state_encryption::*;
// cargo/zigbuild invocations, hence a child process.
#[cfg(not(target_arch = "wasm32"))]
mod cargo_build;
#[cfg(not(target_arch = "wasm32"))]
pub use cargo_build::*;
mod variable;
pub use variable::*;
// mod expression;
// pub use expression::*;
