
publishing
```sh
just build-web
rm -rf /tmp/beet
mkdir -p /tmp/beet || true
cp -r target/static/* /tmp/beet
git checkout pages

mkdir -p play || true
cp -r /tmp/beet/* play

git add .
git commit -m "Publish Playground"
git push origin main
git checkout main

```