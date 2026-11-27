//! AWS's authority over the accounts a package deploys into: who this machine
//! is, how a person elevates or signs in, and the verbs that reach through
//! them. The document, the vault and the `secrets` verbs above are provider
//! neutral; everything AWS shaped lives here.
//!
//! - [`AgentIdentity`]: the global document holding this machine's own aws
//!   pair, and [`AdminSession`], the tmpfs session that elevates it
//! - [`AdminElevate`]: `beet admin`, an administrator session bought with a
//!   code from a phone
//! - [`MemberAccount`]: a package's own account in the operator's Organization,
//!   and the roles that reach it from the management account
//! - [`IdentityCenterClient`] and its [`IdentityCenterSession`]: the human path
//!   into every member account, one browser sign in
//! - [`AwsExec`]: `beet aws`, the aws cli as this machine or a live session

mod admin;
mod agent_identity;
mod aws_exec;
mod identity_center;
mod member_account;

pub use admin::*;
pub use agent_identity::*;
pub use aws_exec::*;
pub use identity_center::*;
pub use member_account::*;
