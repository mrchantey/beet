//! The deploy steps a mail stack takes after its apply.
//!
//! An apply builds a box, a database and a zone full of records, and then stops
//! at the edge of every system that has its own idea of state: the mail server
//! keeps its configuration inside its own data store, the reverse record lives
//! in AWS's PTR service rather than in any zone, and a zone accumulates records
//! nobody declared. These are the steps that close those gaps, and each one is
//! a verb rather than a resource: idempotent, safe to re-run, and reporting
//! what it converged.
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
mod zone_audit;
pub use zone_audit::*;
