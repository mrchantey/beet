use crate::prelude::*;
use anyhow::Result;
use beet_core::prelude::*;
#[cfg(feature = "tokens")]
use proc_macro2::TokenStream;
#[cfg(feature = "tokens")]
use quote::ToTokens;
use std::sync::LazyLock;
use std::sync::Mutex;

#[extend::ext(name=SweetSnapshot)]
pub impl<T, M> T
where
	T: StringComp<M>,
{
	/// Compares the value to a snapshot, saving it if the `--snap` flag is used.
	/// Multiple snapshots per file are supported and indexed by their position.
	/// # Panics
	/// If the snapshot file cannot be read or written.
	#[track_caller]
	fn xpect_snapshot(&self) -> &Self {
		let received = self.to_comp_string();
		match parse_snapshot(&received) {
			Ok(Some(expected)) => {
				panic_ext::assert_diff(&expected, received.into_maybe_not());
			}
			Ok(None) => {
				// snapshot saved, no assertion made
			}
			Err(err) => {
				panic_ext::panic_str(err.to_string());
			}
		}
		self
	}
}

/// Static cache for snapshot data, keyed by source file path.
/// Each entry contains the parsed snapshot call locations and the snapshot values.
struct SnapMap;

impl SnapMap {
	fn cache() -> &'static Mutex<HashMap<WsPathBuf, (Vec<LineCol>, Vec<String>)>>
	{
		static CACHE: LazyLock<
			Mutex<HashMap<WsPathBuf, (Vec<LineCol>, Vec<String>)>>,
		> = LazyLock::new(|| Mutex::new(HashMap::default()));
		&CACHE
	}

	/// Get the snapshot for a given source file and line/column position.
	/// Returns the snapshot string if found, or an error listing available locations.
	fn get(file_path: &WsPathBuf, loc: LineCol) -> Result<String> {
		let mut cache = Self::cache().lock().unwrap();

		// load file data if not cached
		if !cache.contains_key(file_path) {
			let (locs, snapshots) = Self::load_file_data(file_path)?;
			cache.insert(file_path.clone(), (locs, snapshots));
		}

		let (locs, snapshots) = cache.get(file_path).unwrap();

		// find index of this location
		let index = locs.iter().position(|l| *l == loc);

		match index {
			Some(idx) => match snapshots.get(idx).cloned() {
				Some(snapshot) => Ok(snapshot),
				None => {
					let available = locs
						.iter()
						.map(|l| format!("  {}:{}", l.line, l.col))
						.collect::<Vec<_>>()
						.join("\n");
					Err(anyhow::anyhow!(
						"Snapshot at {}:{} exists in parsed locations but has no value.\n\
						Available locations:\n{}\n\
						Please run `cargo test -- --snap` to regenerate snapshots.",
						loc.line,
						loc.col,
						available
					))
				}
			},
			None => {
				let available = locs
					.iter()
					.map(|l| format!("  {}:{}", l.line, l.col))
					.collect::<Vec<_>>()
					.join("\n");
				Err(anyhow::anyhow!(
					"Snapshot location {}:{} not found in parsed locations.\n\
					This likely means the source file has changed since snapshots were generated.\n\
					Available locations:\n{}\n\
					Please run `cargo test -- --snap` to regenerate snapshots.",
					loc.line,
					loc.col,
					available
				))
			}
		}
	}

	/// Set the snapshot for a given source file and line/column position.
	fn set(file_path: &WsPathBuf, loc: LineCol, value: String) -> Result<()> {
		let mut cache = Self::cache().lock().unwrap();

		// Load file data if not cached
		if !cache.contains_key(file_path) {
			let (locs, snapshots) = Self::load_file_data(file_path)?;
			cache.insert(file_path.clone(), (locs, snapshots));
		}

		let (locs, snapshots) = cache.get_mut(file_path).unwrap();

		// find index of this location
		let index = locs.iter().position(|l| *l == loc);

		match index {
			Some(idx) => {
				// extend snapshots vec if needed
				while snapshots.len() <= idx {
					snapshots.push(String::new());
				}
				snapshots[idx] = value;
			}
			None => {
				let available = locs
					.iter()
					.map(|l| format!("  {}:{}", l.line, l.col))
					.collect::<Vec<_>>()
					.join("\n");
				return Err(anyhow::anyhow!(
					"Snapshot location {}:{} not found in parsed locations.\n\
					Available locations:\n{}",
					loc.line,
					loc.col,
					available
				));
			}
		}

		// save updated snapshots
		Self::save_snapshots(file_path, snapshots)?;

		Ok(())
	}

	/// Load file data: parse source for snapshot locations and load existing snapshots.
	fn load_file_data(
		file_path: &WsPathBuf,
	) -> Result<(Vec<LineCol>, Vec<String>)> {
		// parse source file to find all .xpect_snapshot() locations
		let locs = Self::parse_snapshot_locations(file_path)?;

		// load existing snapshots if they exist
		let snap_path = Self::snapshot_path(file_path);
		let snapshots = match fs_ext::read_to_string(&snap_path) {
			Ok(content) => {
				ron::de::from_str::<Vec<String>>(&content).unwrap_or_default()
			}
			Err(FsError::FileNotFound { .. }) => Vec::new(),
			Err(other) => Err(other)?,
		};

		// verify snapshot count matches location count
		if !snapshots.is_empty() && snapshots.len() != locs.len() {
			return Err(anyhow::anyhow!(
				"Snapshot count mismatch for {}:\n\
				Found {} .xpect_snapshot() calls in source but {} snapshots in file.\n\
				Please run `cargo test -- --snap` to regenerate snapshots.",
				file_path,
				locs.len(),
				snapshots.len()
			));
		}

		Ok((locs, snapshots))
	}

	/// Parse the source file to find all `.xpect_snapshot()` call locations.
	/// Returns a vec of LineCol in order of appearance.
	/// Note: col points to the 'x' in 'xpect_snapshot', matching track_caller behavior.
	/// Location::caller() uses tab width of 4 for column calculation.
	fn parse_snapshot_locations(file_path: &WsPathBuf) -> Result<Vec<LineCol>> {
		let abs_path = file_path.into_abs();
		let source = fs_ext::read_to_string(&abs_path).map_err(|err| {
			anyhow::anyhow!("Failed to read source file {}: {}", abs_path, err)
		})?;

		let pattern = ".xpect_snapshot()";
		let mut locations = Vec::new();

		for (line_idx, line_content) in source.lines().enumerate() {
			let line_num = (line_idx + 1) as u32; // 1-indexed

			// find all occurrences of .xpect_snapshot() in this line
			let mut search_start = 0;
			while let Some(pos) = line_content[search_start..].find(pattern) {
				// calculate column with tab expansion (tab width = 4, matching rustc)
				let byte_pos = search_start + pos + 1; // +1 to skip '.' and point to 'x'
				let col_1indexed =
					Self::byte_to_column(&line_content[..byte_pos]);
				// LineCol stores 0-indexed columns, but Location::caller() returns 1-indexed
				let col_0indexed = col_1indexed.saturating_sub(1);
				locations.push(LineCol::new(line_num, col_0indexed));
				search_start += pos + pattern.len();
			}
		}

		Ok(locations)
	}

	/// Convert byte position to column number, expanding tabs to width 4.
	/// This matches the behavior of `Location::caller()`.
	fn byte_to_column(text: &str) -> u32 {
		let mut col = 1u32; // 1-indexed
		for ch in text.chars() {
			if ch == '\t' {
				// tabs advance to next multiple of 4, +1
				col = ((col - 1) / 4 + 1) * 4 + 1;
			} else {
				col += 1;
			}
		}
		col
	}

	/// Get the path where snapshots for a source file are stored.
	fn snapshot_path(file_path: &WsPathBuf) -> AbsPathBuf {
		let file_name = format!(".sweet/snapshots/{}.snap", file_path);
		AbsPathBuf::new_workspace_rel(file_name)
			.expect("Failed to create snapshot path")
	}

	/// Save snapshots to file.
	fn save_snapshots(
		file_path: &WsPathBuf,
		snapshots: &[String],
	) -> Result<()> {
		let snap_path = Self::snapshot_path(file_path);
		let pretty_config = ron::ser::PrettyConfig::default()
			.indentor("  ".to_string())
			.new_line("\n".to_string());
		let content = ron::ser::to_string_pretty(snapshots, pretty_config)?;
		fs_ext::write(&snap_path, &content)?;
		Ok(())
	}
}

/// Parse snapshot - returns Some(expected) if assertion should be made, None if snapshot was saved.
#[allow(dead_code)]
#[track_caller]
fn parse_snapshot(received: &str) -> Result<Option<String>> {
	let caller_loc = core::panic::Location::caller();
	let file_path = WsPathBuf::new(caller_loc.file());
	// Location::caller() returns 1-indexed line and column, but LineCol stores 0-indexed column
	let loc = LineCol::from_location(&caller_loc);

	let args: Vec<String> = std::env::args().collect();
	let is_snap_mode = args.iter().any(|arg| arg == "--snap" || arg == "-s");
	let is_snap_show = args.iter().any(|arg| arg == "--snap-show");

	if is_snap_mode {
		SnapMap::set(&file_path, loc, received.to_string())?;
		beet_core::cross_log!(
			"Snapshot saved: {}:{}:{}",
			file_path,
			loc.line,
			loc.col
		);
		Ok(None)
	} else {
		let snap = SnapMap::get(&file_path, loc)?;
		if is_snap_show {
			beet_core::cross_log!("Snapshot:\n{}", snap);
		}
		Ok(Some(snap))
	}
}

pub trait StringComp<M> {
	fn to_comp_string(&self) -> String;
}

pub struct ToTokensStringCompMarker;

// we dont blanket ToTokens because collision with String
#[cfg(feature = "tokens")]
macro_rules! impl_string_comp_for_tokens {
	($($ty:ty),*) => {
		$(
			impl StringComp<ToTokensStringCompMarker> for $ty {
				fn to_comp_string(&self) -> String {
					pretty_parse(self.to_token_stream())
				}
			}
		)*
	};
}

#[cfg(feature = "tokens")]
impl_string_comp_for_tokens!(
	proc_macro2::TokenStream,
	syn::File,
	syn::Item,
	syn::Expr,
	syn::Stmt,
	syn::Type,
	syn::Pat,
	syn::Ident,
	syn::Block,
	syn::Path,
	syn::Attribute
);

macro_rules! impl_string_comp_for_primitives {
	($($ty:ty),*) => {
		$(
			impl StringComp<$ty> for $ty {
				fn to_comp_string(&self) -> String {
					self.to_string()
				}
			}
		)*
	};
}

impl_string_comp_for_primitives!(
	&str,
	String,
	std::borrow::Cow<'static, str>,
	bool
);

/// Attempt to parse the tokens with prettyplease,
/// otherwise return the tokens as a string.
#[cfg(feature = "tokens")]
pub fn pretty_parse(tokens: TokenStream) -> String {
	use syn::File;
	match syn::parse2::<File>(tokens.clone()) {
		Ok(file) => prettyplease::unparse(&file),
		Err(_) => {
			// ok its not a file, lets try again putting the tokens in a function
			match syn::parse2::<File>(quote::quote! {
				fn deleteme(){
						#tokens
				}
			}) {
				Ok(file) => {
					let mut str = prettyplease::unparse(&file);
					str = str.replace("fn deleteme() {\n", "");
					if let Some(pos) = str.rfind("\n}") {
						str.replace_range(pos..pos + 3, "");
					}
					str =
						str.lines()
							.map(|line| {
								if line.len() >= 4 { &line[4..] } else { line }
							})
							.collect::<Vec<_>>()
							.join("\n");
					str
				}
				Err(_) =>
				// ok still cant parse, just return the tokens as a string
				{
					tokens.to_string()
				}
			}
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[test]
	fn bool() { true.xpect_snapshot(); }

	#[test]
	fn multiple_snapshots_in_one_test() {
		"first".xpect_snapshot();
		"second".xpect_snapshot();
		"third".xpect_snapshot();
	}

	#[cfg(feature = "tokens")]
	#[test]
	fn prettyparse() {
		use quote::quote;
		// valid file
		pretty_parse(quote! {fn main(){let foo = bar;}})
			.xpect_eq("fn main() {\n    let foo = bar;\n}\n");
		pretty_parse(quote! {let foo = bar; let bazz = boo;})
			.xpect_eq("let foo = bar;\nlet bazz = boo;");
	}
}
