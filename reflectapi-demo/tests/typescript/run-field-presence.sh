#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."

if [ "${REFLECTAPI_PRESENCE_SKIP_GENERATION:-}" != 1 ]; then
  cargo test -p reflectapi-demo --test field_presence
fi

if [ -x clients/typescript/node_modules/.bin/tsc ]; then
  typescript=clients/typescript/node_modules/.bin/tsc
else
  typescript=tsc
fi

generated=target/field-presence/typescript
"$typescript" --noEmit --skipLibCheck --strict --target ES2022 --module commonjs \
  --lib esnext,DOM tests/typescript/field-presence.ts \
  "$generated/generated.ts" "$generated/generated.transport.ts"
