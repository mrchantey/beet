# no_std refactor

Refactor the specified crates to be no_std in the order they were provided.

1. Start with no-default-features and get that working with no_std
2. Add features one by one and get them working with no_std

Do not search files the codebase! If you read the crate file by file you will run out of context. the only acceptable way to make changes is to run tests with tail, to preserve context. Only read files that contain errors.


[converting crates to no_std cheatsheet](https://gist.github.com/tdelabro/b2d1f2a0f94ceba72b718b92f9a7ad7b)