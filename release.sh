#!/usr/bin/env bash

set -e

cargo set-version --bump "$1"

TAG=$(cargo metadata --format-version 1 | jq '.packages[] | select(.name == "axum-demo") | .version' --raw-output)

git add Cargo.toml Cargo.lock
git commit -m "Release v${TAG}"
git tag -a "v${TAG}" -m "Release v${TAG}"
git push
git push origin "v${TAG}"
