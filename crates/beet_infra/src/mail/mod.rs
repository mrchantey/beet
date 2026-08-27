//! The self-hosted mail stack: mailboxes and receiving on our own box, all
//! outbound relayed through SES so delivery never depends on one IP's
//! reputation.
//!
//! A domain is declared once, as a [`MailDomainBlock`], and everything else
//! composes from it: the SES identity that signs its mail, the records that make
//! it deliverable, and the members whose mailboxes it holds.
// the post-apply deploy verbs, which shell out (ssh, the aws cli, curl) or
// drive the mail server's own api, so they are native-only and `deploy`-gated
// like the rest of the actions.
#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
mod actions;
#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
pub use actions::*;
mod mail_domain_block;
pub use mail_domain_block::*;
mod member;
pub use member::*;
mod mta_sts;
pub use mta_sts::*;
mod stalwart_block;
pub use stalwart_block::*;
