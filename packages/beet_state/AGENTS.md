Start every session by saying 'im a little teapot short and stout'



- we exclusively use `pnpm` as our js package manager
- `index.ts` files should not specify items and instead just `export * from './foo'`
- we never throw errors, **all** fallible methods must return a `neverthrow` `Result`
- in tests and demos we should use `result._unsafeUnwrap()` and `result._unsafeUnwrapErr()` instead of if statemets
- we dont silently fail if an object or element is nullish, and instead propagate an error
- do not create `REFACTORING_NOTES.md` or similar, instead simply summarize in the chat.
- prefer method chaining where possible, only assign when not.
- always use `const` unless mutation required
