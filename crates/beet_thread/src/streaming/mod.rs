// The streamer *actions* (`O11sStreamer`, the `PostStreamer` dispatch action, the
// tool-call loop and the oneshot entry) run through `beet_action`, so they ride
// `action`; the protocol types and the `PostStreamer` trait itself do not.
#[cfg(feature = "action")]
mod call_functions;
#[cfg(feature = "agent")]
pub mod completions_mapper;
#[cfg(feature = "agent")]
mod completions_streamer;
pub mod o11s_mapper;
#[cfg(feature = "action")]
mod o11s_streamer;
#[cfg(feature = "action")]
mod oneshot;
mod post_streamer;
#[cfg(feature = "action")]
mod post_streamer_action;
#[cfg(feature = "action")]
pub(crate) use call_functions::*;
#[cfg(feature = "agent")]
pub use completions_streamer::*;
#[cfg(feature = "action")]
pub use o11s_streamer::*;
pub use post_streamer::*;
#[cfg(feature = "action")]
pub use post_streamer_action::*;
