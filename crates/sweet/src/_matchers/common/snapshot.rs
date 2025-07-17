use super::*;
use crate::prelude::*;
use anyhow::Result;
use beet_utils::prelude::AbsPathBuf;
use beet_utils::prelude::FsExt;
use beet_utils::prelude::ReadFile;


fn snapshot_location() -> Result<AbsPathBuf> {
	let desc = SweetTestCollector::current_test_desc()
		.ok_or_else(|| anyhow::anyhow!("No current test description found"))?;


	// use test name instead of linecol, which breaks on any file change
	let file_name =
		format!(".sweet/snapshots/{}::{}.ron", desc.source_file, desc.name);

	let abs_save_path = AbsPathBuf::new_workspace_rel(file_name)?;
	Ok(abs_save_path)
}


impl<T: Snapshot> Matcher<T> {
	/// Compares the value to a snapshot, saving it if the `--snapshot` flag is used.
	/// Snapshots are saved using test name so only one snapshot per test is allowed.
	/// # Panics
	/// If the snapshot file cannot be read or written.
	pub fn to_be_snapshot(&self) {
		let save_path = snapshot_location().unwrap();
		let snapshot = self.value.to_snapshot();

		if std::env::args().any(|arg| arg == "--snapshot") {
			FsExt::write(&save_path, snapshot).unwrap();
			println!(
				"Snapshot saved to: {}\nRun tests again without --snapshot to compare",
				save_path
			);
		} else {
			let expected =
				ReadFile::to_string(&save_path).unwrap_or_else(|_| {
					panic!(
						"Snapshot file not found: {}\n
please run test -- --snapshot to generate\n
Snapshots should be commited to version control\n
",
						&save_path
					)
				});
			self.assert_diff(&expected, &snapshot);
		}
	}
}


pub trait Snapshot: Sized {
	fn to_snapshot(&self) -> String;
	fn from_snapshot(snapshot: &str) -> Result<Self, String>;
}

#[cfg(feature = "serde")]
impl<T: 'static + Send + Sync + serde::Serialize + serde::de::DeserializeOwned>
	Snapshot for T
{
	fn to_snapshot(&self) -> String {
		ron::ser::to_string(self).expect("Failed to serialize to snapshot")
	}
	fn from_snapshot(snapshot: &str) -> Result<Self, String> {
		ron::de::from_str(snapshot).map_err(|e| e.to_string())
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[test]
	fn bool() { expect(true).to_be_snapshot(); }
}
