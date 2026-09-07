//! The deploy steps a mail stack takes around its apply.
//!
//! A snapshot gates the apply; then the apply builds a box, its persistent data
//! volume and a zone full of records before stopping at the edge of systems with
//! their own state: the mail server's data store, AWS's PTR service and live DNS.
//! These are verbs rather than resources: idempotent, safe to re-run, and
//! reporting what they converged.
mod comail_deliverability;
pub use comail_deliverability::*;
mod comail_enroll;
pub use comail_enroll::*;
mod dkim_key;
pub use dkim_key::*;
mod eip_reverse_dns;
pub use eip_reverse_dns::*;
mod jmap_client;
pub use jmap_client::*;
mod mail_credentials;
pub use mail_credentials::*;
mod mail_health;
pub use mail_health::*;
mod mail_probe;
pub use mail_probe::*;
mod mail_restore_drill;
pub use mail_restore_drill::*;
mod mail_stack;
pub use mail_stack::*;
mod mta_sts_publish;
pub use mta_sts_publish::*;
mod stalwart_plan;
pub use stalwart_plan::*;
mod stalwart_provision;
pub use stalwart_provision::*;
mod stalwart_snapshot;
pub use stalwart_snapshot::*;
mod zone_audit;
pub use zone_audit::*;
