use limbo::*;


pub struct LimboUtils;

impl LimboUtils {
	pub async fn memory_db() -> Result<limbo::Connection> {
		Builder::new_local(":memory:").build().await?.connect()
	}
}
