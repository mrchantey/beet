# Site docs

How to write and edit the public website docs in `site/routes/docs`. These are the high-level docs at [beet.org](https://beet.org/docs), distinct from the crate READMEs and the rustdoc API docs.

## Audience

READMEs are for people already working with the API. The website is for people forming a mental model. Aim higher-level: explain the shape of things and why they fit together, link out to the READMEs and `docs.rs` for detail.

## Diátaxis first

Load the `diataxis` skill and pick one mode per page before writing. Mixing modes is the main cause of bad docs.

- `docs/index.md`, `docs/crates/*` are **Explanation**: discuss the subject, make connections, give context. Do not instruct.
- `docs/tutorials/*` are **Tutorials**: a guided lesson with a visible result at every step. Do not explain, do not offer choices.

## Tone

- **Be cool, not boastful.** No "beet's central trick", "the whole point", "that is deliberate", "the clearest demonstration". State what it does and move on. Beet has many good ideas; none of them need a fanfare.
- **Never neg other stacks.** Other approaches are valid in their own way. Sell beet on its own terms: malleability and the open Bevy world, not on someone else's expense.
- No em dashes. No mid-sentence line breaks in markdown, only paragraph breaks.

## Conventions

- TOML frontmatter for the title: `+++\ntitle = "Foo"\n+++`.
- Routes mirror the file tree under `site/routes/docs`. A new `foo.md` becomes `/docs/foo`; nested dirs need an `index.md`. The sidebar is auto-collected, no manual wiring.
- Link internally with absolute paths, eg `/docs/crates/beet_router`.
- Use language fences (` ```rust `, ` ```sh `) so syntax highlighting works.
- LLMs love writing text, humans hate reading it. Include exactly what must be there and nothing more. Short code snippets over prose where a snippet says it better.

## Code must be real

A tutorial's contract is that every step works for every reader. Ground all code in something verified:

- Prefer non-`ignore` README doctests and files under `/examples` as the basis.
- Confirm symbols are in the prelude and check the exact feature flags in the root `Cargo.toml` before telling a reader to `cargo add beet --features X`. Default features are `std, ui` only, so most tutorials need to opt in (`action`, `http_server`, `thread`, ...).
- State real prerequisites up front (eg an `OPENAI_API_KEY` for agent tutorials).
