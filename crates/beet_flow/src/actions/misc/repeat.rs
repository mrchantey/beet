use crate::prelude::*;


/// This does **not** trigger observers, making it safe from infinite loops
/// Reattaches the [`RunOnSpawn`] component whenever [`OnRunResult`] is called.
pub type Repeat = InsertOnRunResult<RunOnSpawn>;
