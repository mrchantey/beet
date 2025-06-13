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
}
