# AWS Signature Version 4 test suite

The header signing cases of AWS's SigV4 test suite, as published in the [CRT test suite](https://github.com/awslabs/aws-c-auth/tree/v0.9.0/tests/aws-signing-test-suite) and the `aws-sigv4` crate (which adds `double-encode-path` and `double-url-encode`). Copyright Amazon.com, Inc. or its affiliates, licensed under the Apache License 2.0 (`LICENSE` beside this file).

Only the files a header signature is checked against are kept: `request.txt`, `context.json`, `header-canonical-request.txt`, `header-string-to-sign.txt` and `header-signed-request.txt`. `post-sts-header-after` is left out: it adds the session token unsigned, which no service beet calls asks for.
