#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."

if [ "${REFLECTAPI_ENUM_SKIP_GENERATION:-}" != 1 ]; then
  cargo test -p reflectapi-demo --test enum_compatibility write_generated_clients
fi

if [ -x clients/typescript/node_modules/.bin/tsc ]; then
  typescript=clients/typescript/node_modules/.bin/tsc
else
  typescript=tsc
fi

generated=target/enum-compatibility/typescript
"$typescript" --noEmit --skipLibCheck --strict --target ES2022 --module commonjs \
  --lib esnext,DOM tests/typescript/type-contract.ts \
  "$generated/generated.ts" "$generated/generated.transport.ts"
"$typescript" --skipLibCheck --strict --target ES2022 --module commonjs \
  --lib esnext,DOM --rootDir "$generated" --outDir "$generated/build" \
  "$generated/generated.ts" "$generated/generated.transport.ts"
node --test tests/typescript/runtime.test.cjs
