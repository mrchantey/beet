use crate::prelude::*;


pub type InsertOnRun<T> = InsertOnTrigger<OnRun, T>;
pub type InsertOnRunResult<T> = InsertOnTrigger<OnRunResult, T>;

pub type RemoveOnRun<T> = RemoveOnTrigger<OnRun, T>;
pub type RemoveOnRunResult<T> = RemoveOnTrigger<OnRunResult, T>;
