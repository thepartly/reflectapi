#!/usr/bin/env bash
# Publishes a workspace crate to crates.io unless its version is already there,
# so a release whose publish job failed partway can be re-run.
#
# Usage: publish-crate.sh <package>
set -euo pipefail

package=$1
version=$(cargo metadata --format-version 1 --no-deps \
    | jq -r --arg p "$package" '.packages[] | select(.name == $p) | .version')
if [ -z "$version" ]; then
    echo "::error::$package is not a package in this workspace"
    exit 1
fi

# crates.io asks API clients to identify themselves.
agent="reflectapi release workflow (https://github.com/thepartly/reflectapi)"
status() {
    curl -sS -o /dev/null -w '%{http_code}' -A "$agent" "https://crates.io/api/v1/crates/$1"
}

case "$(status "$package/$version")" in
    200)
        echo "$package $version is already on crates.io; skipping."
        exit 0
        ;;
    404) ;;
    *)
        echo "::error::crates.io did not answer whether $package $version exists"
        exit 1
        ;;
esac

# Trusted publishing can only be set up on a crate that exists, so a new
# crate's first version is published by hand. Say so rather than fail on 403.
if [ "$(status "$package")" = 404 ]; then
    echo "::error::$package is not on crates.io yet. Publish its first version by hand, set up trusted publishing on it, then re-run this job. See CONTRIBUTING.md."
    exit 1
fi

cargo publish --package "$package" --locked
