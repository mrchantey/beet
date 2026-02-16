lets keep iterating on beet_stack!

# `card_walker.rs`

- visit_button is wrong, a button cannot be a TextNode just like html
- for the tui just render buttons a link.
- remove `widgets/button` and `widgets/hyperlink`, just render these directly like in render_markdown.
- Remove the control flow element of the walker, its an antipattern. each usage of it (visit_button, visit_image, even the html one) is a bug, an example of how we've lost the value of the children. instead use leave_button, leave_image to handle appending the suffix.
- ensure that all visit_ and leave_ handlers actually provide a reference to the node being visited/left. this should replace antipatterns like render tui	`current_link_url: Option<String>` (which isnt even being used?)

# `markdown!`

remove this macro, just use mdx directly. use `pkg_ext::internal_or_beet` to resolve crate name. Merge tests and docs.

# undo!

See the current git diff for `parse_markdown.rs` undo these changes, the previous parser was easier to read
