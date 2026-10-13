// Wire decoding is enabled only for generated endpoints whose output or typed
// error can reach an open enum or a serde fallback variant. Descriptors are
// generated from the output schema and preserve unrelated response fields.
type __DecodeSpec = any;
type __DecodeSpecs = Record<string, (args: __DecodeSpec[]) => __DecodeSpec>;

function __decode(value: unknown, spec: __DecodeSpec, specs: __DecodeSpecs): any {
  if (spec === "pass") return value;
  if (spec === "string" || spec === "number" || spec === "boolean") {
    if (typeof value !== spec || (spec === "number" && !Number.isFinite(value))) {
      throw new Error(`expected ${spec}`);
    }
    return value;
  }
  if (spec === "null") {
    if (value !== null) throw new Error("expected null");
    return value;
  }
  switch (spec.k) {
    case "integer":
      if (typeof value !== "number" || !Number.isInteger(value) ||
          (spec.min !== undefined && value < spec.min) ||
          (spec.max !== undefined && value > spec.max)) {
        throw new Error("expected integer in range");
      }
      return value;
    case "ref": {
      if (!Object.prototype.hasOwnProperty.call(specs, spec.n)) {
        throw new Error(`missing decoder for ${spec.n}`);
      }
      const factory = specs[spec.n];
      return __decode(value, factory(spec.a), specs);
    }
    case "nullable":
      return value == null ? value : __decode(value, spec.v, specs);
    case "array":
      if (!Array.isArray(value)) throw new Error("expected array");
      return value.map((item) => __decode(item, spec.v, specs));
    case "tuple":
      if (!Array.isArray(value) || value.length !== spec.v.length) {
        throw new Error("expected tuple");
      }
      return value.map((item, index) => __decode(item, spec.v[index], specs));
    case "map": {
      if (!__is_record(value)) throw new Error("expected object map");
      const result: Record<string, unknown> = {};
      for (const [key, item] of Object.entries(value)) {
        Object.defineProperty(result, key, {
          value: __decode(item, spec.v, specs), enumerable: true, writable: true, configurable: true,
        });
      }
      return result;
    }
    case "object": {
      if (!__is_record(value)) throw new Error("expected object");
      let result: Record<string, unknown> = { ...value };
      for (const [key, field, required] of spec.f) {
        if (!Object.prototype.hasOwnProperty.call(value, key)) {
          if (required) throw new Error(`missing field ${key}`);
          continue;
        }
        Object.defineProperty(result, key, {
          value: __decode(value[key], field, specs), enumerable: true, writable: true, configurable: true,
        });
      }
      for (const flattened of spec.x || []) {
        const optionalTag = __optional_flatten_tag(flattened, specs);
        if (optionalTag !== null && !Object.prototype.hasOwnProperty.call(result, optionalTag)) {
          continue;
        }
        const decoded = __decode(result, flattened, specs);
        // A flattened open enum occupies this same wire object. Keep the
        // enclosing fields when its discriminator is unfamiliar, while its
        // UnknownEnum.raw still points to the original JSON object.
        result = isUnknownEnum(decoded)
          ? { ...result, ...__make_unknown_enum(decoded.tag, value) }
          : decoded;
      }
      return result;
    }
    case "untagged": {
      for (const variant of spec.v) {
        try {
          return __decode(value, variant, specs);
        } catch (_error) {
          // Serde tests untagged variants in declaration order.
        }
      }
      throw new Error("no matching untagged enum variant");
    }
    case "enum":
      return __decode_enum(value, spec, specs);
    default:
      throw new Error("invalid decoder descriptor");
  }
}

function __is_record(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function __optional_flatten_tag(spec: __DecodeSpec, specs: __DecodeSpecs): string | null {
  if (spec?.k !== "nullable") return null;
  let inner = spec.v;
  const seen = new Set<string>();
  while (inner?.k === "ref" && !seen.has(inner.n)) {
    seen.add(inner.n);
    if (!Object.prototype.hasOwnProperty.call(specs, inner.n)) return null;
    inner = specs[inner.n](inner.a);
  }
  return inner?.k === "enum" && inner.r !== "external" ? inner.t : null;
}

function __decode_enum(value: unknown, spec: any, specs: __DecodeSpecs): any {
  let tag: string;
  let payload: unknown;
  if (spec.r === "external") {
    if (typeof value === "string") {
      tag = value;
      payload = undefined;
    } else if (__is_record(value) && Object.keys(value).length === 1) {
      tag = Object.keys(value)[0];
      payload = value[tag];
    } else {
      throw new Error("expected externally tagged enum");
    }
  } else {
    if (!__is_record(value) || typeof value[spec.t] !== "string") {
      throw new Error("expected enum object with string discriminator");
    }
    tag = value[spec.t] as string;
    payload = spec.r === "adjacent" ? value[spec.c] : value;
  }

  const variant = spec.v.find((candidate: any) => candidate.t === tag);
  if (!variant) {
    if (spec.o) return __make_unknown_enum(tag, value);
    if (spec.f !== null) {
      __check_adjacent_unit_content(value, spec);
      return __fallback_enum_value(spec);
    }
    throw new Error(`unknown enum variant ${tag}`);
  }

  if (spec.f !== null && tag === spec.f) {
    __check_adjacent_unit_content(value, spec);
    return __fallback_enum_value(spec);
  }

  if (spec.r === "external" && variant.s === "none") {
    if (typeof value === "string") return value;
    if (payload === null) return tag;
    throw new Error(`expected unit enum variant ${tag}`);
  }
  if (spec.r === "external" && typeof value === "string") {
    throw new Error(`missing payload for enum variant ${tag}`);
  }
  if (spec.r === "adjacent" && variant.s === "none") {
    __check_adjacent_unit_content(value, spec);
    return { [spec.t]: tag };
  }
  if (spec.r === "adjacent" && !Object.prototype.hasOwnProperty.call(value, spec.c)) {
    throw new Error(`missing enum content for ${tag}`);
  }
  if (variant.s === "none") return value;
  const decoded = __decode(payload, variant.d, specs);
  if (spec.r === "external") return { [tag]: decoded };
  if (spec.r === "adjacent") return { ...value as object, [spec.c]: decoded };
  return decoded;
}

function __fallback_enum_value(spec: any): any {
  if (spec.r === "external") return spec.f;
  if (spec.r === "internal") return { [spec.t]: spec.f };
  return { [spec.t]: spec.f };
}

function __check_adjacent_unit_content(value: unknown, spec: any): void {
  if (spec.r === "adjacent" && __is_record(value) &&
      Object.prototype.hasOwnProperty.call(value, spec.c) && value[spec.c] !== null) {
    throw new Error("expected unit variant content");
  }
}
