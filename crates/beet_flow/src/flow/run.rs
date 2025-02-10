use crate::prelude::*;


#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Run;

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RunResult {
	#[default]
	Success,
	Failure,
}


impl ActionPayload for Run {}
impl Request for Run {
	type Res = RunResult;
}

impl ActionPayload for RunResult {}
impl Response for RunResult {
	type Req = Run;
}
