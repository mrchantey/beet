use crate::prelude::*;
use bevy::app::AppExit;

#[extend::ext(name=AppExitExt)]
pub impl AppExit {
	fn anyhow(self) -> anyhow::Result<()> {
		match self {
			AppExit::Success => Ok(()),
			AppExit::Error(err) => {
				Err(anyhow::anyhow!("Application exited with error: {}", err))
			}
		}
	}
	fn into_result(self) -> bevy::prelude::Result {
		match self {
			AppExit::Success => Ok(()),
			AppExit::Error(err) => {
				bevybail!("Application exited with error: {}", err)
			}
		}
	}
}
