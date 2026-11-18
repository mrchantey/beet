use beet_core::prelude::*;

/// A lexicon member no registered parser reads, held whole on the entity
/// it parsed onto, so any renderer reaching the entity writes it back
/// unchanged and a round trip loses nothing.
#[derive(Debug, Clone, PartialEq, Deref, Component, Reflect)]
#[reflect(Component)]
pub struct UnknownLexicon(pub OpenUnion);
