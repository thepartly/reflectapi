import { strict as assert } from "node:assert";
import { test } from "node:test";

import { __request, __stream_request, Err, type Result } from "./generated";
import type { Client, Response } from "./generated.transport";

function respondWith(
  status: number,
  body: string,
  headers: Record<string, string> = {},
): Client {
  return {
    async request(): Promise<Response> {
      const bytes = new TextEncoder().encode(body);
      return {
        status,
        headers: { get: (name: string) => headers[name.toLowerCase()] ?? null },
        body: new ReadableStream({
          start(controller) {
            controller.enqueue(bytes);
            controller.close();
          },
        }),
      };
    },
  };
}

function call(client: Client) {
  return __request<{}, {}, unknown, unknown>(client, "/x", {}, {});
}

test("typed application errors carry status", async () => {
  const err = (await call(respondWith(409, '"Conflict"'))).unwrap_err();
  assert.equal(err.err(), "Conflict");
  assert.equal(err.status_code(), 409);
});

test("non-application errors keep their other_err payload and gain status", async () => {
  const err = (await call(respondWith(503, "upstream connect error"))).unwrap_err();
  assert.equal(err.other_err(), "[503] upstream connect error");
  assert.equal(err.status_code(), 503);
});

test("network failures have no status and a readable toString", async () => {
  const failing: Client = {
    request: () => Promise.reject(new TypeError("fetch failed")),
  };
  const err = (await call(failing)).unwrap_err();
  assert.equal(err.status_code(), undefined);
  assert.equal(err.headers(), undefined);
  assert.equal(err.raw_headers(), undefined);
  assert.ok(err.other_err() instanceof TypeError);
  assert.equal(err.toString(), "Other Error: TypeError: fetch failed");
});

test("a body that fails mid-read keeps the response status", async () => {
  const brokenBody: Client = {
    async request(): Promise<Response> {
      return {
        status: 502,
        headers: { get: () => null },
        body: new ReadableStream({
          start(controller) {
            controller.error(new Error("connection reset"));
          },
        }),
      };
    },
  };
  const err = (await call(brokenBody)).unwrap_err();
  assert.equal(err.status_code(), 502);
});

test("unwrap_ok exposes the Err as cause without changing its message", async () => {
  const result = await call(respondWith(503, "unavailable"));
  assert.throws(
    () => result.unwrap_ok(),
    (thrown: unknown) => {
      assert.ok(thrown instanceof Error);
      assert.equal(
        thrown.message,
        'called `unwrap_ok` on an `err` value: {"value":{"other_err":"[503] unavailable"}}',
      );
      const cause = (thrown as Error & { cause: unknown }).cause;
      assert.ok(cause instanceof Err);
      assert.equal(cause.status_code(), 503);
      assert.equal(Object.getOwnPropertyDescriptor(thrown, "cause")?.enumerable, false);
      assert.equal(JSON.stringify(thrown), "{}");
      return true;
    },
  );
});

test("stream init errors carry status", async () => {
  const result = await __stream_request<{}, {}, unknown, unknown>(
    respondWith(429, "slow down"),
    "/x",
    {},
    {},
  );
  const err = result.unwrap_err();
  assert.equal(err.status_code(), 429);
});

type ResponseHeaders = { "retry-after": string | null; "x-request-id": string | null };
const declared = ["retry-after", "x-request-id"];

test("declared response headers are on successful results, null when absent", async () => {
  const result = await __request<{}, {}, unknown, unknown, ResponseHeaders>(
    respondWith(200, "{}", { "x-request-id": "req-1", "cf-ray": "abc" }),
    "/x",
    {},
    {},
    undefined,
    declared,
  );
  assert.equal(result.status_code(), 200);
  assert.deepEqual(result.headers(), { "x-request-id": "req-1", "retry-after": null });
  assert.equal(result.raw_headers()?.get("cf-ray"), "abc");
  assert.equal(JSON.stringify(result), '{"value":{"ok":{}}}');
  // Existing annotations without the headers type still accept the result.
  const plain: Result<unknown, Err<unknown>> = result;
  assert.ok(plain.is_ok());
});

test("failed results share the response with their Err", async () => {
  const result = await __request<{}, {}, unknown, unknown, ResponseHeaders>(
    respondWith(429, "slow down", { "retry-after": "7" }),
    "/x",
    {},
    {},
    undefined,
    declared,
  );
  const err = result.unwrap_err();
  assert.deepEqual(err.headers(), { "x-request-id": null, "retry-after": "7" });
  assert.deepEqual(result.headers(), err.headers());
  assert.deepEqual(err.map(String).headers(), err.headers());
});
