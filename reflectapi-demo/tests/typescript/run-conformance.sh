#!/bin/sh
# Type-checks serde's conformance samples against the generated TypeScript
# client, then replays them through it. See tests/conformance.rs.
set -eu
cd "$(dirname "$0")/../.."

if [ "${REFLECTAPI_CONFORMANCE_SKIP_GENERATION:-}" != 1 ]; then
  cargo test -p reflectapi-demo --test conformance
fi

if [ -x clients/typescript/node_modules/.bin/tsc ]; then
  typescript=clients/typescript/node_modules/.bin/tsc
else
  typescript=tsc
fi

generated=target/conformance/typescript
cp tests/typescript/conformance.ts "$generated/conformance.ts"
"$typescript" --skipLibCheck --strict --target ES2022 --module commonjs \
  --lib esnext,DOM --types node --typeRoots clients/typescript/node_modules/@types \
  --outDir "$generated/build" \
  "$generated/conformance.ts" "$generated/cases.ts" \
  "$generated/generated.ts" "$generated/generated.transport.ts"
node --test "$generated/build/conformance.js"
