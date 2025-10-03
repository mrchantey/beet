use crate::prelude::*;
use bevy::prelude::*;
use std::process::ExitStatus;





#[extend::ext]
pub impl ExitStatus {
	/// until ExitStatus::exit_ok stablizes
	fn exit_ok(&self) -> Result {
		match self.code() {
			Some(val) if val == 0 => Ok(()),
			Some(val) => bevybail!("Exit with status {val}"),
			None => Ok(()),
		}
	}
}
