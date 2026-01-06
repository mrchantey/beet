use crate::prelude::*;


/// Parses Cli args into request style path and query
// TODO deprecate, just use Parts directly
#[derive(Debug, Clone)]
pub struct CliArgs {
	pub path: Vec<String>,
	pub query: HashMap<String, Vec<String>>,
}


impl CliArgs {
	/// Parses the CLI args from the environment, excluding program name
	pub fn parse_env() -> Self {
		env_ext::args().join(" ").xmap(|val| Self::parse(&val))
	}

	pub fn parse(args: &str) -> Self {
		let args = Self::group_quotations(args);
		let mut path = Vec::new();
		let mut query = HashMap::<String, Vec<String>>::new();
		let mut collecting_nested = false;
		let mut pending_key: Option<String> = None;

		let mut args_iter = args.into_iter();
		while let Some(arg) = args_iter.next() {
			if collecting_nested {
				// After seeing `--`, everything goes into 'nested-args'
				query
					.entry("nested-args".to_string())
					.or_default()
					.push(arg);
			} else if arg == "--" {
				// Start collecting nested args
				// If there's a pending key, it's a flag
				if let Some(key) = pending_key.take() {
					query.entry(key).or_default();
				}
				collecting_nested = true;
			} else if let Some(stripped) =
				arg.strip_prefix("--").or_else(|| arg.strip_prefix("-"))
			{
				// Query param with -- or - prefix
				// If there's a pending key, it's a flag
				if let Some(key) = pending_key.take() {
					query.entry(key).or_default();
				}

				if let Some((key, value)) = stripped.split_once('=') {
					// Key=value format
					query
						.entry(key.to_string())
						.or_default()
						.push(value.to_string());
				} else {
					// No equals sign - might be followed by a value
					pending_key = Some(stripped.to_string());
				}
			} else {
				// Non-dash argument
				if let Some(key) = pending_key.take() {
					// This is the value for the pending key
					query.entry(key).or_default().push(arg);
				} else {
					// Path param
					path.push(arg);
				}
			}
		}

		// Handle any remaining pending key as a flag
		if let Some(key) = pending_key {
			query.entry(key).or_default();
		}

		Self { path, query }
	}

	/// Groups arguments respecting quotations (single and double quotes).
	/// Quotes are stripped from the output. Standard CLI parsing rules:
	/// - Single quotes preserve everything literally (no escape sequences)
	/// - Double quotes allow backslash escaping (\", \\, etc.)
	/// - Outside quotes, backslash escapes the next character
	/// - Unmatched quotes continue to end of input
	fn group_quotations(input: &str) -> Vec<String> {
		let mut result = Vec::new();
		let mut current = String::new();
		let mut in_single_quote = false;
		let mut in_double_quote = false;
		let mut escape_next = false;

		let combined = input;
		let chars: Vec<char> = combined.chars().collect();
		let mut i = 0;

		while i < chars.len() {
			let ch = chars[i];

			if escape_next {
				// Add the escaped character literally
				current.push(ch);
				escape_next = false;
				i += 1;
				continue;
			}

			match ch {
				'\\' if in_double_quote
					|| (!in_single_quote && !in_double_quote) =>
				{
					// Backslash escapes next character in double quotes or outside quotes
					escape_next = true;
					i += 1;
				}
				'\'' if !in_double_quote => {
					// Toggle single quote mode (unless we're in double quotes)
					in_single_quote = !in_single_quote;
					i += 1;
				}
				'"' if !in_single_quote => {
					// Toggle double quote mode (unless we're in single quotes)
					in_double_quote = !in_double_quote;
					i += 1;
				}
				' ' | '\t' | '\n' | '\r'
					if !in_single_quote && !in_double_quote =>
				{
					// Whitespace outside quotes separates arguments
					if !current.is_empty() {
						result.push(current.clone());
						current.clear();
					}
					i += 1;
				}
				_ => {
					// Regular character, add to current argument
					current.push(ch);
					i += 1;
				}
			}
		}

		// Push any remaining content
		if !current.is_empty() {
			result.push(current);
		}

		result
	}

	pub fn into_path_string(&self) -> String {
		let mut path_str = format!("/{}", self.path.join("/"));

		if !self.query.is_empty() {
			let mut first = true;
			for (key, values) in &self.query {
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
		cli.query.is_empty().xpect_true();
	}

	#[test]
	fn parse_single_path() {
		let cli = CliArgs::parse("foo");

		cli.path.len().xpect_eq(1);
		cli.path[0].xpect_eq("foo");
		cli.query.is_empty().xpect_true();
	}

	#[test]
	fn parse_multiple_paths() {
		let cli = CliArgs::parse("foo bar baz");

		cli.path.xpect_eq(vec![
			"foo".to_string(),
			"bar".to_string(),
			"baz".to_string(),
		]);
		cli.query.is_empty().xpect_true();
	}

	#[test]
	fn parse_single_query_param() {
		let cli = CliArgs::parse("--key=value");

		cli.path.xpect_empty();
		cli.query.len().xpect_eq(1);
		cli.query
			.get("key")
			.unwrap()
			.xpect_eq(vec!["value".to_string()]);
	}

	#[test]
	fn parse_query_flag_without_value() {
		let cli = CliArgs::parse("--verbose");

		cli.path.xpect_empty();
		cli.query.len().xpect_eq(1);
		cli.query.get("verbose").unwrap().xpect_empty();
	}

	#[test]
	fn parse_multiple_query_params() {
		let cli = CliArgs::parse("--a=1 --b=2 --c=3");

		cli.path.xpect_empty();
		cli.query.len().xpect_eq(3);
		cli.query.get("a").unwrap().xpect_eq(vec!["1".to_string()]);
		cli.query.get("b").unwrap().xpect_eq(vec!["2".to_string()]);
		cli.query.get("c").unwrap().xpect_eq(vec!["3".to_string()]);
	}

	#[test]
	fn parse_duplicate_query_keys() {
		let cli = CliArgs::parse("--key=val1 --key=val2 --key=val3");

		cli.path.xpect_empty();
		cli.query.len().xpect_eq(1);
		cli.query.get("key").unwrap().xpect_eq(vec![
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
		cli.query.len().xpect_eq(1);
		cli.query
			.get("key")
			.unwrap()
			.xpect_eq(vec!["value".to_string()]);
	}

	#[test]
	fn parse_interleaved_paths_and_query() {
		let cli = CliArgs::parse(
			"path1 --key=val1 path2 --key=val2 --key=val3 --other=one",
		);

		cli.path
			.xpect_eq(vec!["path1".to_string(), "path2".to_string()]);
		cli.query.len().xpect_eq(2);
		cli.query.get("key").unwrap().xpect_eq(vec![
			"val1".to_string(),
			"val2".to_string(),
			"val3".to_string(),
		]);
		cli.query
			.get("other")
			.unwrap()
			.xpect_eq(vec!["one".to_string()]);
	}

	#[test]
	fn parse_empty_value() {
		let cli = CliArgs::parse("--key=");

		cli.path.xpect_empty();
		cli.query.len().xpect_eq(1);
		cli.query.get("key").unwrap().xpect_eq(vec!["".to_string()]);
	}

	#[test]
	fn parse_value_with_equals() {
		let cli = CliArgs::parse("--key=val=ue");

		cli.path.xpect_empty();
		cli.query.len().xpect_eq(1);
		cli.query
			.get("key")
			.unwrap()
			.xpect_eq(vec!["val=ue".to_string()]);
	}

	#[test]
	fn parse_whitespace_separated_value() {
		let cli = CliArgs::parse("--foo bar");

		cli.path.xpect_empty();
		cli.query.len().xpect_eq(1);
		cli.query
			.get("foo")
			.unwrap()
			.xpect_eq(vec!["bar".to_string()]);
	}

	#[test]
	fn parse_single_dash_with_equals() {
		let cli = CliArgs::parse("-f=bar");

		cli.path.xpect_empty();
		cli.query.len().xpect_eq(1);
		cli.query
			.get("f")
			.unwrap()
			.xpect_eq(vec!["bar".to_string()]);
	}

	#[test]
	fn parse_single_dash_whitespace_separated() {
		let cli = CliArgs::parse("-f bar");

		cli.path.xpect_empty();
		cli.query.len().xpect_eq(1);
		cli.query
			.get("f")
			.unwrap()
			.xpect_eq(vec!["bar".to_string()]);
	}

	#[test]
	fn parse_mixed_whitespace_and_equals() {
		let cli = CliArgs::parse("--foo bar --baz=qux");

		cli.path.xpect_empty();
		cli.query.len().xpect_eq(2);
		cli.query
			.get("foo")
			.unwrap()
			.xpect_eq(vec!["bar".to_string()]);
		cli.query
			.get("baz")
			.unwrap()
			.xpect_eq(vec!["qux".to_string()]);
	}

	#[test]
	fn parse_path_then_whitespace_separated_query() {
		let cli = CliArgs::parse("path1 path2 --key value");

		cli.path
			.xpect_eq(vec!["path1".to_string(), "path2".to_string()]);
		cli.query.len().xpect_eq(1);
		cli.query
			.get("key")
			.unwrap()
			.xpect_eq(vec!["value".to_string()]);
	}

	#[test]
	fn parse_flag_before_separator() {
		let cli = CliArgs::parse("--verbose -- nested");

		cli.path.xpect_empty();
		cli.query.len().xpect_eq(2);
		cli.query.get("verbose").unwrap().xpect_empty();
		cli.query
			.get("nested-args")
			.unwrap()
			.xpect_eq(vec!["nested".to_string()]);
	}

	#[test]
	fn parse_multiple_flags_in_sequence() {
		let cli = CliArgs::parse("--verbose --debug --trace");

		cli.path.xpect_empty();
		cli.query.len().xpect_eq(3);
		cli.query.get("verbose").unwrap().xpect_empty();
		cli.query.get("debug").unwrap().xpect_empty();
		cli.query.get("trace").unwrap().xpect_empty();
	}

	#[test]
	fn parse_flag_at_end() {
		let cli = CliArgs::parse("path1 --key=value --flag");

		cli.path.xpect_eq(vec!["path1".to_string()]);
		cli.query.len().xpect_eq(2);
		cli.query
			.get("key")
			.unwrap()
			.xpect_eq(vec!["value".to_string()]);
		cli.query.get("flag").unwrap().xpect_empty();
	}

	#[test]
	fn parse_single_dash_flag() {
		let cli = CliArgs::parse("-v");

		cli.path.xpect_empty();
		cli.query.len().xpect_eq(1);
		cli.query.get("v").unwrap().xpect_empty();
	}

	#[test]
	fn parse_mixed_single_double_dash() {
		let cli = CliArgs::parse("-v --verbose -f bar --foo=baz");

		cli.path.xpect_empty();
		cli.query.len().xpect_eq(4);
		cli.query.get("v").unwrap().xpect_empty();
		cli.query.get("verbose").unwrap().xpect_empty();
		cli.query
			.get("f")
			.unwrap()
			.xpect_eq(vec!["bar".to_string()]);
		cli.query
			.get("foo")
			.unwrap()
			.xpect_eq(vec!["baz".to_string()]);
	}

	#[test]
	fn parse_whitespace_value_looks_like_path() {
		let cli = CliArgs::parse("path1 --key path2 path3");

		cli.path
			.xpect_eq(vec!["path1".to_string(), "path3".to_string()]);
		cli.query.len().xpect_eq(1);
		cli.query
			.get("key")
			.unwrap()
			.xpect_eq(vec!["path2".to_string()]);
	}

	#[test]
	fn parse_nested_args_separator() {
		let cli = CliArgs::parse("foo bar -- nested1 nested2 --flag");

		cli.path
			.xpect_eq(vec!["foo".to_string(), "bar".to_string()]);
		cli.query.len().xpect_eq(1);
		cli.query.get("nested-args").unwrap().xpect_eq(vec![
			"nested1".to_string(),
			"nested2".to_string(),
			"--flag".to_string(),
		]);
	}

	#[test]
	fn parse_nested_args_with_query_before() {
		let cli = CliArgs::parse("foo --name=bob -- arg1 arg2");

		cli.path.xpect_eq(vec!["foo".to_string()]);
		cli.query.len().xpect_eq(2);
		cli.query
			.get("name")
			.unwrap()
			.xpect_eq(vec!["bob".to_string()]);
		cli.query
			.get("nested-args")
			.unwrap()
			.xpect_eq(vec!["arg1".to_string(), "arg2".to_string()]);
	}

	#[test]
	fn parse_only_nested_args() {
		let cli = CliArgs::parse("-- foo bar baz");

		cli.path.xpect_empty();
		cli.query.len().xpect_eq(1);
		cli.query.get("nested-args").unwrap().xpect_eq(vec![
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
		cli.query
			.get("key")
			.unwrap()
			.xpect_eq(vec!["value".to_string()]);
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
		cli.query
			.get("message")
			.unwrap()
			.xpect_eq(vec!["hello world".to_string()]);
	}

	#[test]
	fn parse_with_quoted_equals_value() {
		let cli = CliArgs::parse("--text=\"foo bar\"");
		cli.query
			.get("text")
			.unwrap()
			.xpect_eq(vec!["foo bar".to_string()]);
	}

	#[test]
	fn parse_with_quoted_whitespace_value() {
		let cli = CliArgs::parse("--name 'Jane Doe'");
		cli.query
			.get("name")
			.unwrap()
			.xpect_eq(vec!["Jane Doe".to_string()]);
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
		cli.query
			.get("key")
			.unwrap()
			.xpect_eq(vec!["value with spaces".to_string()]);
	}

	#[test]
	fn parse_str_complex_quoted_command() {
		let cli = CliArgs::parse(
			r#"build --msg="Initial commit" --author='John Doe' src/main.rs"#,
		);
		cli.path
			.xpect_eq(vec!["build".to_string(), "src/main.rs".to_string()]);
		cli.query.len().xpect_eq(2);
		cli.query
			.get("msg")
			.unwrap()
			.xpect_eq(vec!["Initial commit".to_string()]);
		cli.query
			.get("author")
			.unwrap()
			.xpect_eq(vec!["John Doe".to_string()]);
	}
}
