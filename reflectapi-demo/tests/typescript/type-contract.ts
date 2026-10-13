import {
  enum_compatibility,
  isUnknownEnum,
  match,
  unknownEnum,
  type UnknownEnum,
} from "../../target/enum-compatibility/typescript/generated";

type InputOpenInternal = enum_compatibility.fixtures.input.OpenInternal;
type OutputOpenInternal = enum_compatibility.fixtures.output.OpenInternal;
type OutputOpenUnit = enum_compatibility.fixtures.output.OpenUnit;
type OutputOpenExternal = enum_compatibility.fixtures.output.OpenExternal;
type FlattenNamed = enum_compatibility.fixtures.output.FlattenNamedEnvelope;
type FlattenClosedExternal = enum_compatibility.fixtures.output.FlattenClosedExternalEnvelope;
type OptionalFlattenClosedExternal = enum_compatibility.fixtures.output.OptionalFlattenClosedExternalEnvelope;
type ResponseHeaders = enum_compatibility.fixtures.ResponseHeaders;

declare const responseHeaders: ResponseHeaders;
const responseKind = responseHeaders["x-kind"];
if (isUnknownEnum(responseKind)) {
  const futureHeaderTag: string = responseKind[unknownEnum].tag;
  void futureHeaderTag;
} else if (responseKind !== null) {
  const knownHeaderTag: "Ready" | "Unknown" = responseKind;
  void knownHeaderTag;
}

declare const output: OutputOpenInternal;
if (isUnknownEnum(output)) {
  const tag: string = output[unknownEnum].tag;
  const raw: unknown = output[unknownEnum].raw;
  void [tag, raw];
} else if (output.kind === "Dog") {
  const breed: string = output.breed;
  void breed;
}

declare const input: InputOpenInternal;
// @ts-expect-error the synthetic output arm is not part of the request type.
const invalidInput: InputOpenInternal = {} as UnknownEnum;
void [input, invalidInput];

// @ts-expect-error the unique runtime brand cannot be forged structurally.
const unbranded: UnknownEnum = { [unknownEnum]: { tag: "Future", raw: {} } };
void unbranded;

declare const flattened: FlattenNamed;
if (isUnknownEnum(flattened)) {
  const businessTag: string = flattened.tag;
  const businessRaw: string = flattened.raw;
  const futureTag: string = flattened[unknownEnum].tag;
  void [businessTag, businessRaw, futureTag];
}

const flattenedExternalUnit: FlattenClosedExternal = {
  id: 1, ordinary: { kind: "Bird" }, Unit: null,
};
const flattenedExternalValue: FlattenClosedExternal = {
  id: 2, ordinary: { kind: "Bird" }, Value: { message: "known" },
};
const optionalExternalAbsent: OptionalFlattenClosedExternal = {
  id: 3, ordinary: { kind: "Bird" },
};
void [flattenedExternalUnit, flattenedExternalValue, optionalExternalAbsent];

declare const unit: OutputOpenUnit;
const knownOrFuture = match(unit, {
  Ready: () => "ready",
  Unknown: () => "declared Unknown",
  [unknownEnum]: (future) => future.tag,
});
void knownOrFuture;

// @ts-expect-error exhaustive matching must handle the synthetic unknown arm.
match(unit, { Ready: () => "ready", Unknown: () => "declared Unknown" });

match(unit, { Ready: () => "ready" }, (value) => {
  if (isUnknownEnum(value)) return value[unknownEnum].tag;
  return value;
});

declare const external: OutputOpenExternal;
match(external, {
  Unit: () => "unit",
  Value: ({ message }) => message,
  Number: (number) => String(number),
  Tuple: ([number, text]) => `${number}${text}`,
  Optional: (text) => text ?? "empty",
  [unknownEnum]: (future) => future.tag,
});
