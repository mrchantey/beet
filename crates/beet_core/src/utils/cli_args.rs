use crate::prelude::*;


/// Parses CLI args into request-style path and query parameters.
// TODO deprecate, just use Parts directly
#[derive(Debug, Clone)]
pub struct CliArgs {
	/// Positional arguments forming the path.
	pub path: Vec<String>,
	/// Named arguments as key-value pairs, supporting multiple values per key.
	pub params: MultiMap<String, String>,
}


impl CliArgs {
	/// Parses the CLI args from the environment, excluding program name
	pub fn parse_env() -> Self {
		env_ext::args().join(" ").xmap(|val| Self::parse(&val))
	}

	/// Parses CLI arguments from a string.
	pub fn parse(args: &str) -> Self {
		let args = Self::group_quotations(args);
		let mut path = Vec::new();
		let mut params = MultiMap::new();
		let mut collecting_nested = false;
		let mut pending_key: Option<String> = None;

		let mut args_iter = args.into_iter();
		while let Some(arg) = args_iter.next() {
			if collecting_nested {
				// After seeing `--`, everything goes into 'nested-args'
				params.insert("nested-args".to_string(), arg);
			} else if arg == "--" {
				// Start collecting nested args.
				// If there's a pending key, it's a flag.
				if let Some(key) = pending_key.take() {
					params.insert_key(key);
				}
				collecting_nested = true;
			} else if let Some(stripped) =
				arg.strip_prefix("--").or_else(|| arg.strip_prefix("-"))
			{
				// Query param with -- or - prefix.
				// If there's a pending key, it's a flag.
				if let Some(key) = pending_key.take() {
					params.insert_key(key);
				}

				if let Some((key, value)) = stripped.split_once('=') {
					// Key=value format
					params.insert(key.to_string(), value.to_string());
				} else {
					// No equals sign - might be followed by a value
					pending_key = Some(stripped.to_string());
				}
			} else {
				// Non-dash argument
				if let Some(key) = pending_key.take() {
					// This is the value for the pending key
					params.insert(key, arg);
				} else {
					// Path param
					path.push(arg);
				}
			}
		}

		// Handle any remaining pending key as a flag
		if let Some(key) = pending_key {
			params.insert_key(key);
		}

		Self { path, params }
	}

	/// Groups arguments respecting quotations (single and double quotes).
	/// Quotes are stripped from the output. Standard CLI parsing rules:
	/// - Single quotes preserve everything literally (no escape sequences)
	/// - Double quotes allow backslash escaping (\", \\, etc.)
	/// - Outside quotes, backslash escapes the next character
	fn group_quotations(args: &str) -> Vec<String> {
		let mut result = Vec::new();
		let mut current = String::new();
		let mut chars = args.chars().peekable();
		let mut in_single_quote = false;
		let mut in_double_quote = false;

		while let Some(ch) = chars.next() {
			match ch {
				'\'' if !in_double_quote => {
					in_single_quote = !in_single_quote;
				}
				'"' if !in_single_quote => {
					in_double_quote = !in_double_quote;
				}
				'\\' if in_double_quote => {
					// In double quotes, backslash escapes next char
					if let Some(next) = chars.next() {
						match next {
							'"' => current.push('"'),
							'\\' => current.push('\\'),
							'n' => current.push('\n'),
							't' => current.push('\t'),
							_ => {
								current.push('\\');
								current.push(next);
							}
						}
					}
				}
				'\\' if !in_single_quote && !in_double_quote => {
					// Outside quotes, backslash escapes the next character
					if let Some(next) = chars.next() {
						current.push(next);
					}
				}
				c if (c == ' ' || c == '\t' || c == '\n')
					&& !in_single_quote
					&& !in_double_quote =>
				{
					// Whitespace outside quotes is a separator
					if !current.is_empty() {
						result.push(current.clone());
						current.clear();
					}
				}
				c => {
					current.push(c);
				}
			}
		}

		// Push the final token if any
		if !current.is_empty() {
			result.push(current);
		}

		result
	}

	/// Converts the parsed arguments back into a path string with query params.
	pub fn into_path_string(&self) -> String {
		let mut path_str = format!("/{}", self.path.join("/"));

		if !self.params.is_empty() {
			let mut first = true;
			for (key, values) in self.params.iter_all() {
				for value in values {
					if first {
						path_str.push('?');
						first = false;
					} else {
						path_str.push('&');
					}
					path_str.push_str(&format!("{}={}", key, value));
				}
			}
		}
		path_str
	}
}


#[cfg(test)]
mod tests {
	use crate::prelude::*;

	#[test]
	fn parse_empty() {
		let cli = CliArgs::parse("");

		cli.path.xpect_empty();
		cli.params.is_empty().xpect_true();
	}

	#[test]
	fn parse_single_path() {
		let cli = CliArgs::parse("foo");

		cli.path.len().xpect_eq(1);
		cli.path[0].xpect_eq("foo");
		cli.params.is_empty().xpect_true();
	}

	#[test]
	fn parse_multiple_paths() {
		let cli = CliArgs::parse("foo bar baz");

		cli.path.xpect_eq(vec![
			"foo".to_string(),
			"bar".to_string(),
			"baz".to_string(),
		]);
		cli.params.is_empty().xpect_true();
	}

	#[test]
	fn parse_single_query_param() {
		let cli = CliArgs::parse("--key=value");

		cli.path.xpect_empty();
		cli.params.len().xpect_eq(1);
		cli.params.get("key").unwrap().xpect_eq("value");
	}

	#[test]
	fn parse_query_flag_without_value() {
		let cli = CliArgs::parse("--verbose");

		cli.path.xpect_empty();
		cli.params.len().xpect_eq(1);
		cli.params.contains_key("verbose").xpect_true();
		cli.params.get("verbose").xpect_none();
	}

	#[test]
	fn parse_multiple_query_params() {
		let cli = CliArgs::parse("--a=1 --b=2 --c=3");

		cli.path.xpect_empty();
		cli.params.len().xpect_eq(3);
		cli.params.get("a").unwrap().xpect_eq("1");
		cli.params.get("b").unwrap().xpect_eq("2");
		cli.params.get("c").unwrap().xpect_eq("3");
	}

	#[test]
	fn parse_duplicate_query_keys() {
		let cli = CliArgs::parse("--key=val1 --key=val2 --key=val3");

		cli.path.xpect_empty();
		cli.params.len().xpect_eq(1);
		cli.params.get_vec("key").unwrap().xpect_eq(vec![
			"val1".to_string(),
			"val2".to_string(),
			"val3".to_string(),
		]);
	}

	#[test]
	fn parse_mixed_paths_and_query() {
		let cli = CliArgs::parse("foo bar --key=value");

		cli.path
			.xpect_eq(vec!["foo".to_string(), "bar".to_string()]);
		cli.params.len().xpect_eq(1);
		cli.params.get("key").unwrap().xpect_eq("value");
	}

	#[test]
	fn parse_interleaved_paths_and_query() {
		let cli = CliArgs::parse(
			"path1 --key=val1 path2 --key=val2 --key=val3 --other=one",
		);

		cli.path
			.xpect_eq(vec!["path1".to_string(), "path2".to_string()]);
		cli.params.len().xpect_eq(2);
		cli.params.get_vec("key").unwrap().xpect_eq(vec![
			"val1".to_string(),
			"val2".to_string(),
			"val3".to_string(),
		]);
		cli.params.get("other").unwrap().xpect_eq("one");
	}

	#[test]
	fn parse_empty_value() {
		let cli = CliArgs::parse("--key=");

		cli.path.xpect_empty();
		cli.params.len().xpect_eq(1);
		cli.params.get("key").unwrap().xpect_eq("");
	}

	#[test]
	fn parse_value_with_equals() {
		let cli = CliArgs::parse("--key=val=ue");

		cli.path.xpect_empty();
		cli.params.len().xpect_eq(1);
		cli.params.get("key").unwrap().xpect_eq("val=ue");
	}

	#[test]
	fn parse_whitespace_separated_value() {
		let cli = CliArgs::parse("--foo bar");

		cli.path.xpect_empty();
		cli.params.len().xpect_eq(1);
		cli.params.get("foo").unwrap().xpect_eq("bar");
	}

	#[test]
	fn parse_single_dash_with_equals() {
		let cli = CliArgs::parse("-f=bar");

		cli.path.xpect_empty();
		cli.params.len().xpect_eq(1);
		cli.params.get("f").unwrap().xpect_eq("bar");
	}

	#[test]
	fn parse_single_dash_whitespace_separated() {
		let cli = CliArgs::parse("-f bar");

		cli.path.xpect_empty();
		cli.params.len().xpect_eq(1);
		cli.params.get("f").unwrap().xpect_eq("bar");
	}

	#[test]
	fn parse_mixed_whitespace_and_equals() {
		let cli = CliArgs::parse("--foo bar --baz=qux");

		cli.path.xpect_empty();
		cli.params.len().xpect_eq(2);
		cli.params.get("foo").unwrap().xpect_eq("bar");
		cli.params.get("baz").unwrap().xpect_eq("qux");
	}

	#[test]
	fn parse_path_then_whitespace_separated_query() {
		let cli = CliArgs::parse("path1 path2 --key value");

		cli.path
			.xpect_eq(vec!["path1".to_string(), "path2".to_string()]);
		cli.params.len().xpect_eq(1);
		cli.params.get("key").unwrap().xpect_eq("value");
	}

	#[test]
	fn parse_flag_before_separator() {
		let cli = CliArgs::parse("--verbose -- nested");

		cli.path.xpect_empty();
		cli.params.len().xpect_eq(2);
		cli.params.get("verbose").xpect_none();
		cli.params.get("nested-args").unwrap().xpect_eq("nested");
	}

	#[test]
	fn parse_multiple_flags_in_sequence() {
		let cli = CliArgs::parse("--verbose --debug --trace");

		cli.path.xpect_empty();
		cli.params.len().xpect_eq(3);
		cli.params.get("verbose").xpect_none();
		cli.params.get("debug").xpect_none();
		cli.params.get("trace").xpect_none();
	}

	#[test]
	fn parse_flag_at_end() {
		let cli = CliArgs::parse("path1 --key=value --flag");

		cli.path.xpect_eq(vec!["path1".to_string()]);
		cli.params.len().xpect_eq(2);
		cli.params.get("key").unwrap().xpect_eq("value");
		cli.params.get_vec("flag").unwrap().xpect_empty();
	}

	#[test]
	fn parse_single_dash_flag() {
		let cli = CliArgs::parse("-v");

		cli.path.xpect_empty();
		cli.params.len().xpect_eq(1);
		cli.params.get_vec("v").unwrap().xpect_empty();
	}

	#[test]
	fn parse_mixed_single_double_dash() {
		let cli = CliArgs::parse("-v --verbose -f bar --foo=baz");

		cli.path.xpect_empty();
		cli.params.len().xpect_eq(4);
		cli.params.get_vec("v").unwrap().xpect_empty();
		cli.params.get_vec("verbose").unwrap().xpect_empty();
		cli.params.get("f").unwrap().xpect_eq("bar");
		cli.params.get("foo").unwrap().xpect_eq("baz");
	}

	#[test]
	fn parse_whitespace_value_looks_like_path() {
		let cli = CliArgs::parse("path1 --key path2 path3");

		cli.path
			.xpect_eq(vec!["path1".to_string(), "path3".to_string()]);
		cli.params.len().xpect_eq(1);
		cli.params.get("key").unwrap().xpect_eq("path2");
	}

	#[test]
	fn parse_nested_args_separator() {
		let cli = CliArgs::parse("foo bar -- nested1 nested2 --flag");

		cli.path
			.xpect_eq(vec!["foo".to_string(), "bar".to_string()]);
		cli.params.len().xpect_eq(1);
		cli.params.get_vec("nested-args").unwrap().xpect_eq(vec![
			"nested1".to_string(),
			"nested2".to_string(),
			"--flag".to_string(),
		]);
	}

	#[test]
	fn parse_nested_args_with_query_before() {
		let cli = CliArgs::parse("foo --name=bob -- arg1 arg2");

		cli.path.xpect_eq(vec!["foo".to_string()]);
		cli.params.len().xpect_eq(2);
		cli.params.get("name").unwrap().xpect_eq("bob");
		cli.params
			.get_vec("nested-args")
			.unwrap()
			.xpect_eq(vec!["arg1".to_string(), "arg2".to_string()]);
	}

	#[test]
	fn parse_only_nested_args() {
		let cli = CliArgs::parse("-- foo bar baz");

		cli.path.xpect_empty();
		cli.params.len().xpect_eq(1);
		cli.params.get_vec("nested-args").unwrap().xpect_eq(vec![
			"foo".to_string(),
			"bar".to_string(),
			"baz".to_string(),
		]);
	}

	#[test]
	fn into_path_string_empty() {
		let cli = CliArgs::parse("");
		let path_string = cli.into_path_string();

		path_string.xpect_eq("/");
	}

	#[test]
	fn into_path_string_path_only() {
		let cli = CliArgs::parse("foo bar");
		let path_string = cli.into_path_string();

		path_string.xpect_eq("/foo/bar");
	}

	#[test]
	fn into_path_string_query_only() {
		let cli = CliArgs::parse("--a=1 --b=2");
		let path_string = cli.into_path_string();

		path_string.starts_with("/").xpect_true();
		path_string.contains("a=1").xpect_true();
		path_string.contains("b=2").xpect_true();
		path_string.contains('?').xpect_true();
	}

	#[test]
	fn into_path_string_path_and_query() {
		let cli = CliArgs::parse("foo bar --a=1 --b=2");
		let path_string = cli.into_path_string();

		path_string.starts_with("/foo/bar").xpect_true();
		path_string.contains("a=1").xpect_true();
		path_string.contains("b=2").xpect_true();
		path_string.contains('?').xpect_true();
	}

	#[test]
	fn into_path_string_multiple_values_same_key() {
		let cli = CliArgs::parse("foo --key=val1 --key=val2 --key=val3");
		let path_string = cli.into_path_string();

		path_string.starts_with("/foo").xpect_true();
		path_string.contains("key=val1").xpect_true();
		path_string.contains("key=val2").xpect_true();
		path_string.contains("key=val3").xpect_true();
		path_string.contains('?').xpect_true();
		path_string.contains('&').xpect_true();
	}

	#[test]
	fn into_path_string_preserves_empty_value() {
		let cli = CliArgs::parse("foo --key=");
		let path_string = cli.into_path_string();

		path_string.starts_with("/foo").xpect_true();
		path_string.contains("key=").xpect_true();
	}

	#[test]
	fn parse_with_string_from_vec() {
		let cli = CliArgs::parse("foo bar --key=value");

		cli.path
			.xpect_eq(vec!["foo".to_string(), "bar".to_string()]);
		cli.params.get("key").unwrap().xpect_eq("value");
	}

	#[test]
	fn roundtrip_path_string() {
		let original = "api users --limit=10 --offset=20";
		let cli = CliArgs::parse(original);
		let path_string = cli.into_path_string();

		// Should be able to parse it back (though order may differ)
		path_string.starts_with("/api/users").xpect_true();
		path_string.contains("limit=10").xpect_true();
		path_string.contains("offset=20").xpect_true();
	}

	// Tests for group_quotations
	#[test]
	fn group_quotations_empty() {
		let result = CliArgs::group_quotations("");
		result.xpect_empty();
	}

	#[test]
	fn group_quotations_no_quotes() {
		let result = CliArgs::group_quotations("foo bar baz");
		result.xpect_eq(vec![
			"foo".to_string(),
			"bar".to_string(),
			"baz".to_string(),
		]);
	}

	#[test]
	fn group_quotations_single_quotes() {
		let result = CliArgs::group_quotations("foo bar 'bazz boo'");
		result.xpect_eq(vec![
			"foo".to_string(),
			"bar".to_string(),
			"bazz boo".to_string(),
		]);
	}

	#[test]
	fn group_quotations_double_quotes() {
		let result = CliArgs::group_quotations("foo --bar=\"boo bong\"");
		result.xpect_eq(vec!["foo".to_string(), "--bar=boo bong".to_string()]);
	}

	#[test]
	fn group_quotations_single_quotes_multiword() {
		let result = CliArgs::group_quotations("'hello world how are you'");
		result.xpect_eq(vec!["hello world how are you".to_string()]);
	}

	#[test]
	fn group_quotations_mixed_quotes() {
		let result = CliArgs::group_quotations("foo 'bar baz' \"qux quux\"");
		result.xpect_eq(vec![
			"foo".to_string(),
			"bar baz".to_string(),
			"qux quux".to_string(),
		]);
	}

	#[test]
	fn group_quotations_escaped_double_quote() {
		let result = CliArgs::group_quotations("\"hello \\\"world\\\"\"");
		result.xpect_eq(vec!["hello \"world\"".to_string()]);
	}

	#[test]
	fn group_quotations_escaped_backslash() {
		let result = CliArgs::group_quotations("\"path\\\\to\\\\file\"");
		result.xpect_eq(vec!["path\\to\\file".to_string()]);
	}

	#[test]
	fn group_quotations_single_quote_no_escaping() {
		let result = CliArgs::group_quotations("'hello \\world'");
		// Single quotes don't process escapes, backslashes are literal
		result.xpect_eq(vec!["hello \\world".to_string()]);
	}

	#[test]
	fn group_quotations_nested_different_quotes() {
		let result = CliArgs::group_quotations("\"it's fine\"");
		result.xpect_eq(vec!["it's fine".to_string()]);

		let result2 = CliArgs::group_quotations("'say \"hello\"'");
		result2.xpect_eq(vec!["say \"hello\"".to_string()]);
	}

	#[test]
	fn group_quotations_unmatched_quote_continues() {
		let result = CliArgs::group_quotations("'hello world");
		// Unmatched quote continues to end
		result.xpect_eq(vec!["hello world".to_string()]);
	}

	#[test]
	fn group_quotations_multiple_spaces() {
		let result = CliArgs::group_quotations("foo   bar");
		result.xpect_eq(vec!["foo".to_string(), "bar".to_string()]);
	}

	#[test]
	fn group_quotations_quotes_in_middle() {
		let result = CliArgs::group_quotations("--name='John Doe'");
		result.xpect_eq(vec!["--name=John Doe".to_string()]);
	}

	#[test]
	fn group_quotations_empty_quotes() {
		let result = CliArgs::group_quotations("'' \"\"");
		result.xpect_empty();
	}

	#[test]
	fn group_quotations_escaped_outside_quotes() {
		let result = CliArgs::group_quotations("foo\\ bar");
		result.xpect_eq(vec!["foo bar".to_string()]);
	}

	#[test]
	fn group_quotations_complex_command() {
		let result = CliArgs::group_quotations(
			"build --message='Initial commit' --author=\"John Doe\"",
		);
		result.xpect_eq(vec![
			"build".to_string(),
			"--message=Initial commit".to_string(),
			"--author=John Doe".to_string(),
		]);
	}

	// Integration tests with parse
	#[test]
	fn parse_with_quoted_path() {
		let cli = CliArgs::parse("foo 'bar baz'");
		cli.path
			.xpect_eq(vec!["foo".to_string(), "bar baz".to_string()]);
	}

	#[test]
	fn parse_with_quoted_query_value() {
		let cli = CliArgs::parse("--message='hello world'");
		cli.params
			.get_vec("message")
			.unwrap()
			.xpect_eq(vec!["hello world".to_string()]);
	}

	#[test]
	fn parse_with_quoted_equals_value() {
		CliArgs::parse("--text=\"foo bar\"")
			.params
			.get("text")
			.unwrap()
			.xpect_eq("foo bar");
	}

	#[test]
	fn parse_with_quoted_whitespace_value() {
		CliArgs::parse("--name 'Jane Doe'")
			.params
			.get("name")
			.unwrap()
			.xpect_eq("Jane Doe");
	}

	#[test]
	fn group_quotations_consecutive_quoted_args() {
		let result = CliArgs::group_quotations("'foo bar' 'baz qux'");
		result.xpect_eq(vec!["foo bar".to_string(), "baz qux".to_string()]);
	}

	#[test]
	fn group_quotations_quote_then_unquoted() {
		let result = CliArgs::group_quotations("'hello world' normal");
		result.xpect_eq(vec!["hello world".to_string(), "normal".to_string()]);
	}

	#[test]
	fn group_quotations_special_chars_in_quotes() {
		let result = CliArgs::group_quotations("'foo=bar&baz=qux'");
		result.xpect_eq(vec!["foo=bar&baz=qux".to_string()]);
	}

	#[test]
	fn group_quotations_tab_and_newline() {
		let result = CliArgs::group_quotations("foo\tbar baz\nqux");
		// Tabs and newlines act as separators outside quotes
		result.xpect_eq(vec![
			"foo".to_string(),
			"bar".to_string(),
			"baz".to_string(),
			"qux".to_string(),
		]);
	}

	#[test]
	fn group_quotations_preserve_whitespace_in_quotes() {
		let result = CliArgs::group_quotations("'foo\tbar\nbaz'");
		result.xpect_eq(vec!["foo\tbar\nbaz".to_string()]);
	}

	#[test]
	fn group_quotations_escape_space() {
		let result = CliArgs::group_quotations("foo\\ bar\\ baz");
		result.xpect_eq(vec!["foo bar baz".to_string()]);
	}

	#[test]
	fn group_quotations_mixed_escape_and_quotes() {
		let result = CliArgs::group_quotations("foo\\ bar 'baz qux'");
		result.xpect_eq(vec!["foo bar".to_string(), "baz qux".to_string()]);
	}

	#[test]
	fn parse_str_with_quotes() {
		let cli = CliArgs::parse("foo 'bar baz' --key='value with spaces'");
		cli.path
			.xpect_eq(vec!["foo".to_string(), "bar baz".to_string()]);
		cli.params.get("key").unwrap().xpect_eq("value with spaces");
	}

	#[test]
	fn parse_str_complex_quoted_command() {
		let cli = CliArgs::parse(
			r#"build --msg="Initial commit" --author='John Doe' src/main.rs"#,
		);
		cli.path
			.xpect_eq(vec!["build".to_string(), "src/main.rs".to_string()]);
		cli.params.len().xpect_eq(2);
		cli.params.get("msg").unwrap().xpect_eq("Initial commit");
		cli.params.get("author").unwrap().xpect_eq("John Doe");
	}
}
