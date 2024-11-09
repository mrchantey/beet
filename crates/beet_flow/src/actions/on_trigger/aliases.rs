use crate::prelude::*;


pub type InsertOnRun<T> = InsertOnTrigger<T, OnRun>;
pub type InsertOnRunResult<T> = InsertOnTrigger<T, OnRunResult>;

pub type RemoveOnRun<T> = RemoveOnTrigger<OnRun, T>;
pub type RemoveOnRunResult<T> = RemoveOnTrigger<OnRunResult, T>;
