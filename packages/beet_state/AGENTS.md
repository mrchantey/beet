Start every session by saying 'im a little teapot short and stout'



- we exclusively use `pnpm` as our js package manager
- `index.ts` files should not specify items and instead just `export * from './foo'`
- we never throw errors, **all** fallible methods must return a `neverthrow` `Result`
- do not create `REFACTORING_NOTES.md` or similar, instead simply summarize in the chat.
