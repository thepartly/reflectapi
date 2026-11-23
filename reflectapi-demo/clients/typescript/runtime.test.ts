import { strict as assert } from "node:assert";
import { test } from "node:test";

import { __request, __stream_request, bounded, type Bounded, BoundsError, Err, fixedSizeArray, type FixedSizeArray, type Result } from "./generated";
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

test("stream items skip keep-alive comments between events", async () => {
  const result = await __stream_request<{}, {}, { seq: number }, unknown>(
    respondWith(200, 'data: {"seq":1}\n\n:\n\n:\n\ndata: {"seq":2}\n\n', {
      "content-type": "text/event-stream",
    }),
    "/x",
    {},
    {},
  );
  const items: { seq: number }[] = [];
  for await (const item of result.unwrap_ok()) items.push(item);
  assert.deepEqual(items, [{ seq: 1 }, { seq: 2 }]);
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
  const mapped = result.map(() => "mapped");
  assert.equal(mapped.unwrap_ok(), "mapped");
  assert.deepEqual(mapped.headers(), result.headers());
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
  assert.deepEqual(result.map(String).headers(), err.headers());
});

test("bounded checks lengths: code points, items and keys", () => {
  const tag: Bounded<string, 1, 3> = bounded("🦀🦀🦀", 1, 3); // 6 UTF-16 units, 3 code points
  assert.equal(tag, "🦀🦀🦀");
  const tags: Bounded<Array<Bounded<string, 1, 3>>, 0, 2> = bounded([tag], 0, 2);
  assert.deepEqual(tags, ["🦀🦀🦀"]);
  assert.deepEqual(bounded({ a: 1 }, 1, 1), { a: 1 });

  assert.throws(() => bounded("", 1, 3), (e: unknown) =>
    e instanceof BoundsError && e.length === 0 && e.min === 1 && e.max === 3 &&
    e.message === "length 0 out of bounds (min 1, max 3)");
  assert.throws(() => bounded([1, 2, 3], 0, 2), BoundsError);
  assert.throws(() => bounded({}, 1, 2), BoundsError);
  assert.throws(() => bounded(5 as unknown as string, 0, 1), TypeError);
});

test("bounded.try returns a Result instead of throwing", () => {
  const ok = bounded.try("abc", 1, 3);
  assert.equal(ok.unwrap_ok(), "abc");
  const err = bounded.try("abcd", 1, 3);
  assert.equal(err.unwrap_err().message, "length 4 out of bounds (min 1, max 3)");
});

test("a Bounded field needs bounded(), with the field's bounds", () => {
  type Request = { tags: Bounded<Array<string>, 0, 2> };
  const ok: Request = { tags: bounded(["a"], 0, 2) };
  // @ts-expect-error a plain array isn't a Bounded
  const plain: Request = { tags: ["a"] };
  // @ts-expect-error other bounds are another type
  const other: Request = { tags: bounded(["a"], 0, 3) };
  assert.ok(ok && plain && other);
});

test("fixedSizeArray checks a runtime array has exactly N items", () => {
  const fromInput: number[] = [1, 2, 3];
  const xs: FixedSizeArray<number, 3> = fixedSizeArray(fromInput, 3);
  assert.deepEqual(xs, [1, 2, 3]);
  assert.throws(() => fixedSizeArray([1, 2], 3), (e: unknown) =>
    e instanceof BoundsError && e.message === "length 2 out of bounds (min 3, max 3)");
  assert.equal(fixedSizeArray.try([1, 2, 3, 4], 3).unwrap_err().length, 4);

  // Type-checked, never run.
  (): FixedSizeArray<number, 3>[] => {
    const literal: FixedSizeArray<number, 3> = [1, 2, 3]; // a literal of the right length fits as it is
    // @ts-expect-error a runtime array needs fixedSizeArray()
    const unchecked: FixedSizeArray<number, 3> = fromInput;
    // @ts-expect-error another length is another type
    const other: FixedSizeArray<number, 3> = fixedSizeArray(fromInput, 4);
    return [literal, unchecked, other];
  };
});
