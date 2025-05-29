# Beet Commands


## say-hi

> says hi
```sh
echo 'hi'
```

## init-repo

> Initialize the repository, pulling assets into their respective crates. Also we need to build the test_site codegen, which cant use a build script due to cyclic dependencies

```sh
mask init-flow
mask init-rsx
```

## init-rsx

> Once beet-cli is binstallable we shouldnt need to compile in order to codegen

```sh
just cli build -p beet_site
cd infra && npm ci
mkdir -p target/lambda/crates/beet_site || true
echo 'dummy file so sst deploys' > target/lambda/crates/beet_site/bootstrap
```

## init-sweet

> Install chromedriver for testing

```sh
just install-chromedriver
```

## init-flow

> All commands required to initialize `beet_flow` and friends.

```sh
mask assets-pull
mkdir -p ws_flow/beet_ml/assets/ml && cp ./assets/ml/default-bert.ron ws_flow/beet_ml/assets/ml/default.bert.ron
```

## assets-pull

> Pulls assets directory from s3 if `./assets` doesn't exist.


```sh
if [ -d "./assets" ]; then
	echo "skipping assets pull, assets directory already exists"
else
	curl -o ./assets.tar.gz https://bevyhub-public.s3.us-west-2.amazonaws.com/assets.tar.gz
	tar -xzvf ./assets.tar.gz
	rm ./assets.tar.gz
fi
``



