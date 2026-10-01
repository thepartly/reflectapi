import { strict as assert } from "node:assert";
import { test } from "node:test";

import { __request, __stream_request, Err } from "./generated";
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

test("typed application errors carry status and headers", async () => {
  const result = await call(
    respondWith(409, '"Conflict"', { "x-request-id": "abc" }),
  );
  const err = result.unwrap_err();
  assert.equal(err.err(), "Conflict");
  assert.equal(err.status_code(), 409);
  assert.equal(err.metadata()?.headers.get("x-request-id"), "abc");
});

test("non-application errors keep their other_err payload and gain status", async () => {
  const result = await call(
    respondWith(503, "upstream connect error", { "retry-after": "7" }),
  );
  const err = result.unwrap_err();
  assert.equal(err.other_err(), "[503] upstream connect error");
  assert.equal(err.status_code(), 503);
  assert.equal(err.metadata()?.headers.get("retry-after"), "7");
});

test("network failures have no status and a readable toString", async () => {
  const failing: Client = {
    request: () => Promise.reject(new TypeError("fetch failed")),
  };
  const err = (await call(failing)).unwrap_err();
  assert.equal(err.status_code(), undefined);
  assert.equal(err.metadata(), undefined);
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
    respondWith(429, "slow down", { "retry-after": "2" }),
    "/x",
    {},
    {},
  );
  const err = result.unwrap_err();
  assert.equal(err.status_code(), 429);
  assert.equal(err.metadata()?.headers.get("retry-after"), "2");
});
