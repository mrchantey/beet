//! Sentence-similarity Bert (MiniLM family) built on Burn.
//!
//! See [`Bert`] for the model + tokenizer container, [`BertConfig`] for
//! HuggingFace-compatible configuration, and [`nearest_sentence`] for
//! the action that selects the child sentence most similar to the
//! agent's prompt.
mod backend;
pub use self::backend::*;
mod bert_asset;
pub use self::bert_asset::*;
mod bert_config;
pub use self::bert_config::*;
mod bert_loader;
pub use self::bert_loader::*;
mod bert_embedding;
pub(crate) use self::bert_embedding::*;
mod bert_model;
pub use self::bert_model::*;
mod nearest_sentence;
pub use self::nearest_sentence::*;
mod pooling;
pub use self::pooling::*;
#[cfg(feature = "spatial")]
mod sentence_steer_target;
#[cfg(feature = "spatial")]
pub use self::sentence_steer_target::*;
mod tokenize;
pub use self::tokenize::*;
