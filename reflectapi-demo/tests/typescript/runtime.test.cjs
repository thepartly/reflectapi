const assert = require("node:assert/strict");
const test = require("node:test");
const sdk = require("../../target/enum-compatibility/typescript/build/generated.js");
const fallbackSamples = require("../../target/enum-compatibility/fallback_samples.json");

function transport(wire, status = 200, contentType = "application/json", responseHeaders = {}) {
  const body = contentType === "text/event-stream" ? wire : JSON.stringify(wire);
  return {
    request: async () => ({
      status,
      headers: new Headers({ "content-type": contentType, ...responseHeaders }),
      body: new Response(body).body,
    }),
  };
}

async function call(method, wire, status = 200, responseHeaders = {}) {
  return sdk.client(transport(wire, status, "application/json", responseHeaders)).enums[method]({}, {});
}

test("declared response headers decode open enums", async () => {
  const future = await call("internal", { kind: "Bird" }, 200, { "x-kind": "Future" });
  assert.equal(future.is_ok(), true);
  const unknown = future.headers()["x-kind"];
  assert.equal(sdk.isUnknownEnum(unknown), true);
  assert.equal(unknown.tag, "Future");
  assert.equal(unknown.raw, "Future");
  assert.equal(future.raw_headers().get("x-kind"), "Future");

  const known = await call("internal", { kind: "Bird" }, 200, { "x-kind": "Ready" });
  assert.equal(known.headers()["x-kind"], "Ready");

  const missing = await call("internal", { kind: "Bird" });
  assert.equal(missing.headers()["x-kind"], null);
});

test("open output enums retain unfamiliar tags and raw payloads", async () => {
  const future = { kind: "Fox", nocturnal: true };
  const result = await call("internal", future);
  assert.equal(result.is_ok(), true);
  const value = result.unwrap_ok();
  assert.equal(sdk.isUnknownEnum(value), true);
  assert.equal(value.tag, "Fox");
  assert.deepEqual(value.raw, future);

  const adjacent = { kind: "Future", data: { arbitrary: [1, 2] } };
  const adjacentResult = await call("adjacent", adjacent);
  assert.equal(sdk.isUnknownEnum(adjacentResult.unwrap_ok()), true);
  assert.deepEqual(adjacentResult.unwrap_ok().raw, adjacent);

  const external = { Future: { arbitrary: true } };
  const externalResult = await call("external", external);
  assert.equal(sdk.isUnknownEnum(externalResult.unwrap_ok()), true);
  assert.deepEqual(externalResult.unwrap_ok().raw, external);

  for (const tag of ["constructor", "toString", "__proto__"]) {
    const internal = (await call("internal", { kind: tag })).unwrap_ok();
    assert.equal(sdk.isUnknownEnum(internal), true);
    assert.equal(internal.tag, tag);
    const object = JSON.parse(`{"${tag}":{"future":true}}`);
    const externalObject = (await call("external", object)).unwrap_ok();
    assert.equal(sdk.isUnknownEnum(externalObject), true);
    assert.equal(externalObject.tag, tag);
    assert.deepEqual(externalObject.raw, object);
  }
});

test("known wire tags retain their generated shapes and validate payloads", async () => {
  assert.deepEqual((await call("internal", { kind: "Dog", breed: "Husky" })).unwrap_ok(), {
    kind: "Dog", breed: "Husky",
  });
  assert.deepEqual((await call("external", { Number: 3 })).unwrap_ok(), { Number: 3 });
  assert.deepEqual((await call("external", { Tuple: [3, "x"] })).unwrap_ok(), { Tuple: [3, "x"] });
  assert.equal((await call("external", { Unit: null })).unwrap_ok(), "Unit");
  assert.equal((await call("units", { Ready: null })).unwrap_ok(), "Ready");
  assert.equal((await call("units", { Unknown: null })).unwrap_ok(), "Unknown");
  assert.deepEqual((await call("adjacent", { kind: "Unit", data: null })).unwrap_ok(), { kind: "Unit" });

  for (const [method, wire] of [
    ["internal", { kind: "Dog" }],
    ["internal", { kind: "Dog", breed: 3 }],
    ["internal", { breed: "Husky" }],
    ["adjacent", { kind: "Value" }],
    ["adjacent", { kind: "Value", data: { message: 3 } }],
    ["adjacent", { kind: "Unit", data: {} }],
    ["external", { Value: { message: 3 } }],
    ["external", { Number: "three" }],
    ["external", { Number: -1 }],
    ["external", { Number: 3.5 }],
    ["external", { Number: 4294967296 }],
    ["external", { Tuple: [3] }],
    ["external", { A: 1, B: 2 }],
    ["external", { Unit: true }],
    ["units", { Ready: 1 }],
  ]) {
    const result = await call(method, wire);
    assert.equal(result.is_err(), true, `${method}: ${JSON.stringify(wire)}`);
    assert.equal(result.unwrap_err().is_other_err(), true);
  }
});

test("a legitimate Unknown wire variant cannot collide with the match fallback", async () => {
  const known = (await call("units", "Unknown")).unwrap_ok();
  const future = (await call("units", "Future")).unwrap_ok();
  assert.equal(known, "Unknown");
  assert.equal(sdk.isUnknownEnum(known), false);
  assert.equal(sdk.isUnknownEnum(future), true);
  const cases = {
    Ready: () => "ready",
    Unknown: () => "declared",
    [sdk.unknownEnum]: (unknown) => unknown.tag,
  };
  assert.equal(sdk.match(known, cases), "declared");
  assert.equal(sdk.match(future, cases), "Future");
  assert.equal(sdk.match(future, {}, (unknown) => unknown.tag), "Future");
});

test("nested containers, generic values and aliases decode recursively", async () => {
  const future = { kind: "Future", extra: { x: 1 } };
  const envelope = {
    internal: future,
    adjacent: { kind: "Future", data: [1, 2] },
    external: "Future",
    units: "Ready",
    generic: { kind: "Value", value: "ok" },
    fallback: { kind: "Soon", message: "discard" },
    adjacent_fallback: { kind: "Soon", data: null },
    open_with_fallback: { kind: "Soon", value: true },
    optional: future,
    list: [future, { kind: "Bird" }],
    map: JSON.parse('{"first":{"kind":"Future","extra":{"x":1}},"__proto__":{"kind":"Future"}}'),
    alias: future,
    extra_field: 7,
  };
  const result = await call("echo", envelope);
  assert.equal(result.is_ok(), true);
  const value = result.unwrap_ok();
  for (const item of [value.internal, value.adjacent, value.external, value.open_with_fallback,
    value.optional, value.list[0], value.map.first, value.alias]) {
    assert.equal(sdk.isUnknownEnum(item), true);
  }
  assert.deepEqual(value.list[1], { kind: "Bird" });
  assert.deepEqual(value.generic, { kind: "Value", value: "ok" });
  assert.deepEqual(value.fallback, { kind: "other" });
  assert.deepEqual(value.adjacent_fallback, { kind: "Other" });
  assert.equal(value.extra_field, 7);
  assert.equal(Object.prototype.hasOwnProperty.call(value.map, "__proto__"), true);
  assert.equal(sdk.isUnknownEnum(value.map["__proto__"]), true);

  const malformedFallback = await call("adjacent_fallback", { kind: "Soon", data: { no: true } });
  assert.equal(malformedFallback.unwrap_err().is_other_err(), true);
});

test("flattened fields preserve earlier enum conversions and unknown fields", async () => {
  const wire = {
    ordinary: "FutureOrdinary",
    first: "FutureFirst",
    second: { kind: "FutureSecond", detail: 8 },
    extra_field: "preserved",
  };
  const result = await call("flatten", wire);
  assert.equal(result.is_ok(), true);
  const value = result.unwrap_ok();
  for (const [field, tag, raw] of [
    ["ordinary", "FutureOrdinary", "FutureOrdinary"],
    ["first", "FutureFirst", "FutureFirst"],
    ["second", "FutureSecond", wire.second],
  ]) {
    assert.equal(sdk.isUnknownEnum(value[field]), true, field);
    assert.equal(value[field].tag, tag);
    assert.deepEqual(value[field].raw, raw);
  }
  assert.equal(value.extra_field, "preserved");
});

test("a directly flattened open enum retains enclosing fields", async () => {
  const futureWire = { id: 7, kind: "Future", detail: true };
  const future = (await call("flatten_enum", futureWire)).unwrap_ok();
  assert.equal(sdk.isUnknownEnum(future), true);
  assert.equal(future.tag, "Future");
  assert.equal(future.id, 7);
  assert.deepEqual(future.raw, futureWire);

  assert.deepEqual(
    (await call("flatten_enum", { id: 8, kind: "Dog", breed: "Husky" })).unwrap_ok(),
    { id: 8, kind: "Dog", breed: "Husky" },
  );
  const malformed = await call("flatten_enum", { id: 9, kind: "Dog", breed: 3 });
  assert.equal(malformed.unwrap_err().is_other_err(), true);
});

test("an omitted optional flattened enum leaves enclosing fields intact", async () => {
  const absent = (await call("optional_flatten_enum", { id: 7, extra: "preserved" })).unwrap_ok();
  assert.deepEqual(absent, { id: 7, extra: "preserved" });

  const futureWire = { id: 8, kind: "Future", extra: "preserved" };
  const future = (await call("optional_flatten_enum", futureWire)).unwrap_ok();
  assert.equal(sdk.isUnknownEnum(future), true);
  assert.equal(future.id, 8);
  assert.equal(future.extra, "preserved");
  assert.deepEqual(future.raw, futureWire);

  assert.deepEqual(
    (await call("optional_flatten_enum", { id: 9, kind: "Dog", breed: "Husky" })).unwrap_ok(),
    { id: 9, kind: "Dog", breed: "Husky" },
  );
  const malformed = await call("optional_flatten_enum", { id: 10, kind: "Dog", breed: 3 });
  assert.equal(malformed.unwrap_err().is_other_err(), true);
});

test("untagged wrappers decode reachable open enums in order", async () => {
  const future = { kind: "Future", detail: 9 };
  const untagged = (await call("untagged", future)).unwrap_ok();
  assert.equal(sdk.isUnknownEnum(untagged), true);
  assert.deepEqual(untagged.raw, future);
  assert.equal((await call("untagged", "text")).unwrap_ok(), "text");
  assert.deepEqual((await call("untagged", { kind: "Bird" })).unwrap_ok(), { kind: "Bird" });

  const nested = (await call("nested", { kind: "Wrapped", value: future })).unwrap_ok();
  assert.equal(sdk.isUnknownEnum(nested.value), true);
  assert.deepEqual(nested.value.raw, future);
  const malformed = await call("untagged", { kind: "Dog", breed: 9 });
  assert.equal(malformed.unwrap_err().is_other_err(), true);
});

test("serde fallback accepts and rejects the source-backed wire samples", async () => {
  for (const [representation, method] of [["internal", "fallback"], ["adjacent", "adjacent_fallback"]]) {
    for (const sample of fallbackSamples[representation]) {
      const result = await call(method, sample.wire);
      if (sample.ok) {
        assert.equal(result.is_ok(), true, `${representation}: ${JSON.stringify(sample.wire)}`);
        if (sample.value.kind === "other" || sample.value.kind === "Other") {
          assert.deepEqual(result.unwrap_ok(), sample.value);
        }
      } else {
        assert.equal(result.is_err(), true, `${representation}: ${JSON.stringify(sample.wire)}`);
        assert.equal(result.unwrap_err().is_other_err(), true);
      }
    }
  }
});

test("typed application errors and SSE items decode open enums", async () => {
  const error = await call("error", { kind: "Future", detail: 9 }, 400);
  assert.equal(error.is_err(), true);
  assert.equal(error.unwrap_err().is_err(), true);
  const typed = error.unwrap_err().err();
  assert.equal(sdk.isUnknownEnum(typed), true);
  assert.equal(typed.tag, "Future");

  const stream = await sdk.client(transport(
    'data: {"kind":"Future","detail":9}\n\ndata: {"kind":"Bird"}\n\n',
    200,
    "text/event-stream",
  )).enums.stream({}, {});
  assert.equal(stream.is_ok(), true);
  const items = [];
  for await (const item of stream.unwrap_ok()) items.push(item);
  assert.equal(sdk.isUnknownEnum(items[0]), true);
  assert.deepEqual(items[1], { kind: "Bird" });

  const broken = await sdk.client(transport(
    'data: {"kind":"Dog","breed":9}\n\n', 200, "text/event-stream",
  )).enums.stream({}, {});
  await assert.rejects(async () => {
    for await (const _ of broken.unwrap_ok()) { /* consume */ }
  }, /SSE parse error/);
});
