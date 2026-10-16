import { field_presence } from "../../target/field-presence/typescript/generated";

type Input = field_presence.fixtures.input.Fields;
type Output = field_presence.fixtures.output.Fields;

const omitted: Input = { custom: null };
const explicit: Input = {
  plain: null, custom: null, defaulted: null, serializer_only: null,
  skipped: null, three_state: null, default_three_state: null,
};
// @ts-expect-error a custom deserializer without a default requires its key.
const missingCustom: Input = {};
const output: Output = {
  plain: null, custom: null, defaulted: null, serializer_only: null,
  three_state: null, default_three_state: null,
};
// @ts-expect-error ordinary output Option is required even when its value is null.
const missingPlain: Output = {
  custom: null, defaulted: null, serializer_only: null,
  three_state: null, default_three_state: null,
};
// @ts-expect-error a serializer-only override does not make its output omittable.
const missingSerializer: Output = {
  plain: null, custom: null, defaulted: null,
  three_state: null, default_three_state: null,
};
const variant: field_presence.fixtures.input.Variant = { kind: "Value" };
// @ts-expect-error an output struct variant's nullable field is required.
const missingVariant: field_presence.fixtures.output.Variant = { kind: "Value" };
const flat: field_presence.fixtures.input.Flat = { id: 1, custom: null };
// @ts-expect-error flatten preserves required nullable output fields.
const missingFlat: field_presence.fixtures.output.Flat = { id: 1, custom: null };
const generic: field_presence.fixtures.input.Wrapper<string> = {};
// @ts-expect-error Option<T> is nullable but required on output.
const missingWrapper: field_presence.fixtures.output.Wrapper<string> = {};
const tuple: field_presence.fixtures.Tuple = [null, null];
// @ts-expect-error tuple positions cannot be omitted merely because they are nullable.
const missingTuple: field_presence.fixtures.Tuple = [];
const equal: field_presence.fixtures.Equal = {};
void [omitted, explicit, missingCustom, output, missingPlain, missingSerializer,
  variant, missingVariant, flat, missingFlat, generic, missingWrapper, tuple, missingTuple, equal];

const defaults: field_presence.fixtures.input.Defaults = {};
// @ts-expect-error an optional non-nullable field accepts omission, not null.
const nullDefault: field_presence.fixtures.input.Defaults = { count: null };
// @ts-expect-error a default on deserialization does not omit its output key.
const missingDefault: field_presence.fixtures.output.Defaults = {};
void [defaults, nullDefault, missingDefault];
