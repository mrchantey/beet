//! The self-hosted mail stack: mailboxes and receiving on our own box, and
//! outbound leaving through whichever relay each domain composes beside it:
//! Amazon SES, comail, or none at all.
//!
//! A domain is declared once, as a [`MailDomainBlock`], and everything else
//! composes from it: the records that make it deliverable, the sovereign key
//! that signs its mail whatever carries it, and the members whose mailboxes it
//! holds. The relay is the one thing NOT a field on it: `{SesRelay}` beside the
//! domain buys Amazon's reputation, `{ComailRelay}` rides an atproto identity,
//! and nothing at all delivers straight to each recipient's MX (see
//! [`RelayMode`]).
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
mod relay;
pub use relay::*;
mod stalwart_block;
pub use stalwart_block::*;
