+++
title= "Docs"
+++

## Quickstart
In this quick start we will create a new website, deploy it to aws lambda and remove it. 

### Build Dependencies

```sh
# install prebuilt binaries
cargo install cargo-binstall
# used by beet new
cargo binstall cargo-generate
# building wasm
cargo binstall wasm-opt wasm-bindgen-cli --version=0.2.100
```

### Deploy Dependencies

- [AWS credentials](./web/infra.md#aws-credentials) must be configured for the infrastructure steps.

```sh
# configuring infrastructure
npm i -g sst
# deploying to lambda (using a script also configures zig)
curl -fsSL https://cargo-lambda.info/install.sh | sh
```

### Setup

We can begin by setting up a new application:

```sh
cargo binstall beet
beet new && cd beet_new_web
beet run
```

### Deploy

Its time to share our wonderful creation with the world! Clicking around the aws console is error-prone and difficult to reproduce, instead we'll use the `sst` Infrastructure as Code tool to manage all of this for us.

```sh
beet infra
```

Our lambda function is live, but clicking on it we get an `Internal Server Error`. This is because sst deployed an empty `bootstrap`, lets update it with our site:

```sh
beet deploy
```

Now visiting the lambda we should have a working web app with client interactivity and server functions. Now we're done lets remove all resources.

```sh
beet infra remove
```

