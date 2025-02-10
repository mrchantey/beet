use crate::prelude::*;

pub type OnRun = OnRequest<Run>;
pub type OnRunResult = OnResponse<Run>;

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Run;

impl Request for Run {
	type Res = Result<(), ()>;
}
