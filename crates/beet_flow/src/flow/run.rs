use crate::prelude::*;


#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Run;

impl Request for Run {
	type Res = Result<(), ()>;
}
