// Calls each conformance route of the generated TypeScript client with
// serde's samples, through a transport that records the request and serves
// the sample back. `cargo test -p reflectapi-demo --test conformance` writes
// the client, `cases.ts` and `samples.json`; `run-conformance.sh` copies
// this file beside them.
import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { cases } from "./cases";
import { client } from "./generated";
import type { Client, Request, Response } from "./generated.transport";

type Divergence = {
  case: string;
  sample: number | null;
  target: string;
  check: string;
  reason: string;
};

const divergences: Divergence[] = JSON.parse(
  readFileSync(`${__dirname}/../../samples.json`, "utf8"),
).divergences;

function divergence(name: string, sample: number, check: string): string | undefined {
  return divergences.find(
    (d) =>
      d.case === name &&
      d.target === "typescript" &&
      d.check === check &&
      (d.sample === null || d.sample === sample),
  )?.reason;
}

/**
 * JSON text as a value whose numbers are kept as written, so that an integer
 * a client rounds (a `u64` above 2^53, say) compares unequal to serde's.
 * Floats compare by value, since serde and JavaScript spell some differently
 * (`1e300` and `1e+300`).
 */
function exact(text: string): unknown {
  return JSON.parse(text, (_key, value, context?: { source: string }) => {
    if (typeof value !== "number") return value;
    const source = context!.source;
    return /^-?\d+$/.test(source) ? { integer: BigInt(source).toString() } : { float: value };
  });
}

function echo(sample: string): { client: Client; sent: () => Request | undefined } {
  let sent: Request | undefined;
  return {
    client: {
      async request(request: Request): Promise<Response> {
        sent = request;
        const bytes = new TextEncoder().encode(sample);
        return {
          status: 200,
          headers: { get: () => null },
          body: new ReadableStream({
            start(controller) {
              controller.enqueue(bytes);
              controller.close();
            },
          }),
        };
      },
    },
    sent: () => sent,
  };
}

/** Expects `run` to fail exactly when the sample is a known divergence. */
async function conform(name: string, i: number, check: string, run: () => Promise<void>) {
  const reason = divergence(name, i, check);
  if (reason === undefined) return run();
  try {
    await run();
  } catch {
    return;
  }
  assert.fail(`listed as a divergence (${reason}) but passes; remove it from DIVERGENCES`);
}

for (const { name, path, samples, call } of cases) {
  samples.forEach((sample, i) => {
    test(`${name}-${i} request is the sample`, () =>
      conform(name, i, "request", async () => {
        const transport = echo(sample);
        await call(client(transport.client).conformance, JSON.parse(sample));
        const sent = transport.sent();
        assert.ok(sent, "no request was sent");
        assert.equal(sent.path, path);
        assert.deepStrictEqual(exact(new TextDecoder().decode(sent.body)), exact(sample));
      }));

    test(`${name}-${i} response re-encodes as the sample`, () =>
      conform(name, i, "response", async () => {
        const transport = echo(sample);
        const result = await call(client(transport.client).conformance, JSON.parse(sample));
        assert.deepStrictEqual(exact(JSON.stringify(result.unwrap_ok())), exact(sample));
      }));
  });
}
