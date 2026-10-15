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

test("flattened fallback preserves a declared __proto__ field", async () => {
  const wire = JSON.parse('{"id":7,"__proto__":{"safe":1},"kind":"Future"}');
  const value = (await call("flatten_proto_fallback", wire)).unwrap_ok();
  assert.equal(Object.getPrototypeOf(value), Object.prototype);
  assert.equal(Object.prototype.hasOwnProperty.call(value, "__proto__"), true);
  assert.deepEqual(value.__proto__, { safe: 1 });
  assert.deepEqual(value, JSON.parse('{"id":7,"__proto__":{"safe":1},"kind":"other"}'));
});

test("declared response headers decode open enums", async () => {
  const future = await call("internal", { kind: "Bird" }, 200, { "x-kind": "Future" });
  assert.equal(future.is_ok(), true);
  const unknown = future.headers()["x-kind"];
  assert.equal(sdk.isUnknownEnum(unknown), true);
  assert.equal(unknown[sdk.unknownEnum].tag, "Future");
  assert.equal(unknown[sdk.unknownEnum].raw, "Future");
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
  assert.equal(value[sdk.unknownEnum].tag, "Fox");
  assert.deepEqual(value[sdk.unknownEnum].raw, future);
  assert.deepEqual(JSON.parse(JSON.stringify(value)), future);

  const adjacent = { kind: "Future", data: { arbitrary: [1, 2] } };
  const adjacentResult = await call("adjacent", adjacent);
  assert.equal(sdk.isUnknownEnum(adjacentResult.unwrap_ok()), true);
  assert.deepEqual(adjacentResult.unwrap_ok()[sdk.unknownEnum].raw, adjacent);

  const external = { Future: { arbitrary: true } };
  const externalResult = await call("external", external);
  assert.equal(sdk.isUnknownEnum(externalResult.unwrap_ok()), true);
  assert.deepEqual(externalResult.unwrap_ok()[sdk.unknownEnum].raw, external);

  for (const tag of ["constructor", "toString", "__proto__"]) {
    const internal = (await call("internal", { kind: tag })).unwrap_ok();
    assert.equal(sdk.isUnknownEnum(internal), true);
    assert.equal(internal[sdk.unknownEnum].tag, tag);
    const object = JSON.parse(`{"${tag}":{"future":true}}`);
    const externalObject = (await call("external", object)).unwrap_ok();
    assert.equal(sdk.isUnknownEnum(externalObject), true);
    assert.equal(externalObject[sdk.unknownEnum].tag, tag);
    assert.deepEqual(externalObject[sdk.unknownEnum].raw, object);
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
  assert.equal(sdk.match(future, {}, (unknown) => unknown[sdk.unknownEnum].tag), "Future");
  assert.equal(JSON.stringify(future), '"Future"');
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
    optional: future,
    list: [future, { kind: "Bird" }],
    map: JSON.parse('{"first":{"kind":"Future","extra":{"x":1}},"__proto__":{"kind":"Future"}}'),
    alias: future,
    extra_field: 7,
  };
  const result = await call("echo", envelope);
  assert.equal(result.is_ok(), true);
  const value = result.unwrap_ok();
  for (const item of [value.internal, value.adjacent, value.external,
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
    assert.equal(value[field][sdk.unknownEnum].tag, tag);
    assert.deepEqual(value[field][sdk.unknownEnum].raw, raw);
  }
  assert.equal(value.extra_field, "preserved");
});

test("a directly flattened open enum retains enclosing fields", async () => {
  const futureWire = { id: 7, kind: "Future", detail: true };
  const future = (await call("flatten_enum", futureWire)).unwrap_ok();
  assert.equal(sdk.isUnknownEnum(future), true);
  assert.equal(future[sdk.unknownEnum].tag, "Future");
  assert.equal(future.id, 7);
  assert.deepEqual(future[sdk.unknownEnum].raw, futureWire);

  assert.deepEqual(
    (await call("flatten_enum", { id: 8, kind: "Dog", breed: "Husky" })).unwrap_ok(),
    { id: 8, kind: "Dog", breed: "Husky" },
  );
  const malformed = await call("flatten_enum", { id: 9, kind: "Dog", breed: 3 });
  assert.equal(malformed.unwrap_err().is_other_err(), true);
});

test("a flattened open enum preserves declared tag and raw fields", async () => {
  const wire = { id: 7, tag: "business-tag", raw: "business-raw", kind: "Future", detail: true };
  const value = (await call("flatten_named", wire)).unwrap_ok();
  assert.equal(sdk.isUnknownEnum(value), true);
  assert.equal(value.tag, "business-tag");
  assert.equal(value.raw, "business-raw");
  assert.equal(value[sdk.unknownEnum].tag, "Future");
  assert.deepEqual(value[sdk.unknownEnum].raw, wire);
  assert.deepEqual(JSON.parse(JSON.stringify(value)), wire);
  assert.equal(Object.getOwnPropertyDescriptor(value, sdk.unknownEnum).enumerable, false);

  const known = { id: 8, tag: "business-tag", raw: "business-raw", kind: "Dog", breed: "Husky" };
  assert.deepEqual((await call("flatten_named", known)).unwrap_ok(), known);
});

test("nested flatten keeps symbol metadata through both object decoders", async () => {
  const wire = { ordinary: "note", id: 7, kind: "Future", detail: true };
  const value = (await call("nested_flatten", wire)).unwrap_ok();
  assert.equal(sdk.isUnknownEnum(value), true);
  assert.equal(value.ordinary, "note");
  assert.equal(value.id, 7);
  assert.equal(value[sdk.unknownEnum].tag, "Future");
  assert.deepEqual(value[sdk.unknownEnum].raw, wire);
  assert.deepEqual(JSON.parse(JSON.stringify(value)), wire);
});

test("flattened closed external enums decode beside an open field", async () => {
  const future = { kind: "Future", detail: true };
  for (const method of ["flatten_closed_external", "optional_flatten_closed_external"]) {
    const unitWire = { id: 7, ordinary: future, Unit: null };
    const unit = (await call(method, unitWire)).unwrap_ok();
    assert.deepEqual(JSON.parse(JSON.stringify(unit)), unitWire);
    assert.equal(sdk.isUnknownEnum(unit.ordinary), true);
    assert.equal(unit.ordinary[sdk.unknownEnum].tag, "Future");

    const valueWire = { id: 8, ordinary: future, Value: { message: "known" } };
    const value = (await call(method, valueWire)).unwrap_ok();
    assert.deepEqual(JSON.parse(JSON.stringify(value)), valueWire);
    assert.equal(sdk.isUnknownEnum(value.ordinary), true);

    const malformed = await call(method, { id: 9, ordinary: future, Value: { message: 3 } });
    assert.equal(malformed.unwrap_err().is_other_err(), true);
    const ambiguous = await call(method, { id: 10, ordinary: future, Unit: null, Value: { message: "known" } });
    assert.equal(ambiguous.unwrap_err().is_other_err(), true);
  }
  const absent = (await call("optional_flatten_closed_external", {
    id: 11, ordinary: future,
  })).unwrap_ok();
  assert.equal(sdk.isUnknownEnum(absent.ordinary), true);
  assert.equal(Object.prototype.hasOwnProperty.call(absent, "Unit"), false);
  const missing = await call("flatten_closed_external", { id: 12, ordinary: future });
  assert.equal(missing.unwrap_err().is_other_err(), true);
});

test("flattened adjacent open enums keep parents and canonicalize known units", async () => {
  const futureWire = { id: 7, kind: "Future", data: { detail: true } };
  const future = (await call("flatten_adjacent", futureWire)).unwrap_ok();
  assert.equal(sdk.isUnknownEnum(future), true);
  assert.equal(future.id, 7);
  assert.deepEqual(future[sdk.unknownEnum].raw, futureWire);
  assert.deepEqual(JSON.parse(JSON.stringify(future)), futureWire);

  const unit = (await call("flatten_adjacent", { id: 8, kind: "Unit", data: null })).unwrap_ok();
  assert.deepEqual(unit, { id: 8, kind: "Unit" });
  const malformed = await call("flatten_adjacent", { id: 9, kind: "Value", data: { message: 3 } });
  assert.equal(malformed.unwrap_err().is_other_err(), true);

  assert.deepEqual((await call("optional_flatten_adjacent", { id: 10 })).unwrap_ok(), { id: 10 });
  const optional = (await call("optional_flatten_adjacent", { id: 11, kind: "Future", data: 4 })).unwrap_ok();
  assert.equal(sdk.isUnknownEnum(optional), true);
  assert.equal(optional.id, 11);
  assert.deepEqual(optional[sdk.unknownEnum].raw, { id: 11, kind: "Future", data: 4 });
});

test("an omitted optional flattened enum leaves enclosing fields intact", async () => {
  const absent = (await call("optional_flatten_enum", { id: 7, extra: "preserved" })).unwrap_ok();
  assert.deepEqual(absent, { id: 7, extra: "preserved" });

  const futureWire = { id: 8, kind: "Future", extra: "preserved" };
  const future = (await call("optional_flatten_enum", futureWire)).unwrap_ok();
  assert.equal(sdk.isUnknownEnum(future), true);
  assert.equal(future.id, 8);
  assert.equal(future.extra, "preserved");
  assert.deepEqual(future[sdk.unknownEnum].raw, futureWire);

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
  assert.deepEqual(untagged[sdk.unknownEnum].raw, future);
  assert.equal((await call("untagged", "text")).unwrap_ok(), "text");
  assert.deepEqual((await call("untagged", { kind: "Bird" })).unwrap_ok(), { kind: "Bird" });

  const nested = (await call("nested", { kind: "Wrapped", value: future })).unwrap_ok();
  assert.equal(sdk.isUnknownEnum(nested.value), true);
  assert.deepEqual(nested.value[sdk.unknownEnum].raw, future);
  const malformed = await call("untagged", { kind: "Dog", breed: 9 });
  assert.equal(malformed.unwrap_err().is_other_err(), true);
});

test("serde fallback accepts and rejects the source-backed wire samples", async () => {
  for (const [representation, method] of [
    ["internal", "fallback"], ["adjacent", "adjacent_fallback"], ["external", "external_fallback"],
    ["flatten_internal", "flatten_fallback"], ["flatten_adjacent", "flatten_adjacent_fallback"],
  ]) {
    for (const sample of fallbackSamples[representation]) {
      const result = await call(method, sample.wire);
      if (sample.ok) {
        assert.equal(result.is_ok(), true, `${representation}: ${JSON.stringify(sample.wire)}`);
        if (typeof sample.value === "string" ||
            sample.value.kind === "other" || sample.value.kind === "Other") {
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
  assert.equal(typed[sdk.unknownEnum].tag, "Future");

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
