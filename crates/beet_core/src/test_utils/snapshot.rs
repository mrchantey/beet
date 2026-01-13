use crate::prelude::*;
use core::panic::Location;
#[cfg(feature = "tokens")]
use proc_macro2::TokenStream;
#[cfg(feature = "tokens")]
use quote::ToTokens;
use std::fmt::Debug;
use std::sync::LazyLock;
use std::sync::Mutex;

#[extend::ext(name=SweetDebugSnapshot)]
pub impl<T: Debug> T {
	/// Converts to formatted debug string and then creates
	/// a snapshot, see [`SweetSnapshot::xpect_snapshot`]
	#[track_caller]
	fn xpect_debug_snapshot(&self) -> &Self {
		self.xfmt().xpect_snapshot();
		self
	}
}
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
		match parse_snapshot(&received, Location::caller()) {
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
/// Each entry contains tuples of (LineCol, String) for each snapshot.
struct SnapMap;

impl SnapMap {
	fn cache() -> &'static Mutex<MultiMap<WsPathBuf, (LineCol, String)>> {
		static CACHE: LazyLock<Mutex<MultiMap<WsPathBuf, (LineCol, String)>>> =
			LazyLock::new(|| Mutex::new(MultiMap::default()));
		&CACHE
	}

	/// Get the snapshot for a given source file and line/column position.
	/// Returns the snapshot string if found, or an error listing available locations.
	fn get(file_path: &WsPathBuf, loc: LineCol) -> Result<String> {
		// load file data if not cached
		Self::init_key(file_path)?;
		let cache = Self::cache().lock().unwrap();
		let entries = cache.get_vec(file_path).expect("key initialized");

		// find entry with matching location
		let entry = entries.iter().find(|(l, _)| *l == loc);

		match entry {
			Some((_, snapshot)) => Ok(snapshot.clone()),
			None => {
				let available = entries
					.iter()
					.map(|(l, _)| format!("  {}:{}", l.line, l.col))
					.collect::<Vec<_>>()
					.join("\n");
				bevybail!(
					"Snapshot location {}:{} not found in parsed locations.\n\
					This likely means the source file has changed since snapshots were generated.\n\
					Available locations:\n{}\n\
					Please run `cargo test -- --snap` to regenerate snapshots.",
					loc.line,
					loc.col,
					available
				)
			}
		}
	}

	/// Set the snapshot for a given source file and line/column position.
	fn set(file_path: &WsPathBuf, loc: LineCol, value: String) -> Result<()> {
		Self::init_key(file_path)?;

		// find entry with matching location
		let mut cache = Self::cache().lock().unwrap();
		let entries = cache.get_vec_mut(file_path).expect("key initialized");
		let entry_idx = entries.iter().position(|(l, _)| *l == loc);

		match entry_idx {
			Some(idx) => {
				entries[idx] = (loc, value);
			}
			None => {
				let available = entries
					.iter()
					.map(|(l, _)| format!("  {}:{}", l.line, l.col))
					.collect::<Vec<_>>()
					.join("\n");
				bevybail!(
					"Snapshot location {}:{} not found in parsed locations.\n\
					Available locations:\n{}",
					loc.line,
					loc.col,
					available
				);
			}
		}

		// save updated snapshots
		Self::save_snapshots(file_path, &entries)?;

		Ok(())
	}

	fn init_key(file_path: &WsPathBuf) -> Result {
		let mut cache = Self::cache().lock().unwrap();
		// load file data if not cached
		if !cache.contains_key(file_path) {
			let entries = Self::load_file_data(file_path)?;
			cache.insert_vec(file_path.clone(), entries);
		}
		Ok(())
	}

	/// Load file data: parse source for snapshot locations and load existing snapshots.
	fn load_file_data(file_path: &WsPathBuf) -> Result<Vec<(LineCol, String)>> {
		// parse source file to find all .xpect_snapshot() locations
		let locs = Self::parse_snapshot_locations(file_path)?;

		// load existing snapshots if they exist
		let snap_dir = Self::snapshot_path(file_path);
		let entries = locs
			.iter()
			.enumerate()
			.map(|(i, loc)| {
				let snap_file = snap_dir.join(format!("{}.snap", i + 1));
				let content = fs_ext::read_to_string(&snap_file)
					.unwrap_or_else(|_| String::new());
				(*loc, content)
			})
			.collect();

		Ok(entries)
	}

	/// Parse the source file to find all `.xpect_snapshot()` call locations.
	/// Returns a vec of LineCol in order of appearance.
	/// Note: col points to the 'x' in 'xpect_snapshot', matching track_caller behavior.
	/// Location::caller() uses tab width of 4 for column calculation.
	fn parse_snapshot_locations(file_path: &WsPathBuf) -> Result<Vec<LineCol>> {
		let abs_path = file_path.into_abs();
		let source = fs_ext::read_to_string(&abs_path).map_err(|err| {
			bevyhow!("Failed to read source file {}: {}", abs_path, err)
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

	/// Get the directory path where snapshots for a source file are stored.
	fn snapshot_path(file_path: &WsPathBuf) -> AbsPathBuf {
		let dir_name = format!(".sweet/snapshots/{}", file_path);
		AbsPathBuf::new_workspace_rel(dir_name)
			.expect("Failed to create snapshot path")
	}

	/// Save snapshots to individual files.
	fn save_snapshots(
		file_path: &WsPathBuf,
		snapshots: &[(LineCol, String)],
	) -> Result<()> {
		let snap_dir = Self::snapshot_path(file_path);

		// create directory if it doesn't exist
		fs_ext::create_dir_all(&snap_dir)?;

		// write each snapshot to its own file (1.snap, 2.snap, etc.)
		for (idx, (_linecol, snapshot)) in snapshots.iter().enumerate() {
			let snap_file = snap_dir.join(format!("{}.snap", idx + 1));
			fs_ext::write(&snap_file, snapshot)?;
		}

		Ok(())
	}
}

enum SnapMode {
	Save,
	Show,
	Test,
}
impl SnapMode {
	fn parse() -> Self {
		let args = env_ext::args();
		let contains = |flag: &str| args.iter().any(|arg| arg == flag);
		if contains("--snap-show") {
			Self::Show
		} else if contains("--snap") || contains("-s") {
			Self::Save
		} else {
			Self::Test
		}
	}
}

/// Parse snapshot - returns Some(expected) if assertion should be made, None if snapshot was saved.
fn parse_snapshot(
	received: &str,
	caller_loc: &Location,
) -> Result<Option<String>> {
	let file_path = WsPathBuf::new(caller_loc.file());
	// Location::caller() returns 1-indexed line and column, but LineCol stores 0-indexed column
	let loc = LineCol::from_location(&caller_loc);


	match SnapMode::parse() {
		SnapMode::Save => {
			SnapMap::set(&file_path, loc, received.to_string())?;
			crate::cross_log!(
				"Snapshot saved: {}:{}:{}",
				file_path,
				loc.line,
				loc.col
			);
			Ok(None)
		}
		SnapMode::Show => {
			let snap = SnapMap::get(&file_path, loc)?;
			crate::cross_log!("Snapshot:\n{}", snap);
			Ok(None)
		}
		SnapMode::Test => SnapMap::get(&file_path, loc)?.xsome().xok(),
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

impl_string_comp_for_primitives!(&str, String, std::borrow::Cow<'static, str>);

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

// libtest doesnt allow us to pass the --snap option,
// so use _sweet_runner feature with --snap for setting snapshots
#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[test]
	fn once() { "foobar".xpect_snapshot(); }

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
