use super::*;
use crate::prelude::*;
use anyhow::Result;
use beet_utils::prelude::AbsPathBuf;
use beet_utils::prelude::FsExt;
use beet_utils::prelude::ReadFile;
#[cfg(feature = "tokens")]
use quote::ToTokens;


// returns whether the assertion should be made
fn parse_snapshot(received: &str) -> Result<Option<String>> {
	let desc = SweetTestCollector::current_test_desc()
		.ok_or_else(|| anyhow::anyhow!("No current test description found"))?;

	// use test name instead of linecol, which would no longer match on any line/col shifts
	let file_name =
		format!(".sweet/snapshots/{}::{}.ron", desc.source_file, desc.name);

	let save_path = AbsPathBuf::new_workspace_rel(file_name)?;

	if std::env::args().any(|arg| arg == "--snapshot") {
		FsExt::write(&save_path, received)?;
		println!(
			"Snapshot saved: {}\nRun tests again without --snapshot to compare",
			desc.name
		);
		Ok(None)
	} else {
		let expected = ReadFile::to_string(&save_path).unwrap_or_else(|_| {
			panic!(
				"Snapshot file not found: {}\n
				please run test -- --snapshot to generate\n
				Snapshots should be commited to version control\n
				",
				&save_path
			)
		});
		Ok(Some(expected))
	}
}

// #[cfg(feature = "tokens")]
// impl<T: ToTokens> Matcher<T> {

// }

impl<T> Matcher<T> {
	/// Compares the value to a snapshot, saving it if the `--snapshot` flag is used.
	/// Snapshots are saved using test name so only one snapshot per test is allowed.
	/// # Panics
	/// If the snapshot file cannot be read or written.
	pub fn to_be_snapshot<M>(&self)
	where
		T: StringComp<M>,
	{
		let received = self.value.to_comp_string();
		if let Some(expected) = parse_snapshot(&received).unwrap() {
			self.assert_diff(&expected, &received);
		}
	}
}


pub trait StringComp<M> {
	fn to_comp_string(&self) -> String;
}

#[cfg(feature = "serde")]
impl<T: serde::Serialize> StringComp<Self> for T {
	fn to_comp_string(&self) -> String {
		ron::ser::to_string(&self).expect("Failed to serialize to string")
	}
}

pub struct ToTokensStringCompMarker;

#[cfg(feature = "tokens")]
impl<T: ToTokens> StringComp<ToTokensStringCompMarker> for T {
	fn to_comp_string(&self) -> String {
		// TODO format nicely
		self.to_token_stream().to_string()
	}
}

#[cfg(not(feature = "serde"))]
impl<T: ToString> StringComp<Self> for Matcher<T> {
	fn to_comp_string(&self) -> String { self.value.to_string() }
}


#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[derive(serde::Serialize)]
	struct MyStruct(u32);

	#[test]
	fn bool() { expect(MyStruct(7)).to_be_snapshot(); }
}
