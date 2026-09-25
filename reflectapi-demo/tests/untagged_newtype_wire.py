"""Exercise generated models, not handwritten stand-ins, against serde JSON shapes."""

import json

from pydantic import BaseModel, TypeAdapter, ValidationError

from codegen_coverage_client.codegen_coverage.coverage import UntaggedNewtype

adapter = TypeAdapter(UntaggedNewtype)
for wire in ["example", 42, {"label": "named variant"}]:
    parsed = adapter.validate_json(json.dumps(wire))
    assert json.loads(adapter.dump_json(parsed)) == wire


class Envelope(BaseModel):
    value: UntaggedNewtype


assert json.loads(Envelope.model_validate({"value": "example"}).model_dump_json()) == {
    "value": "example"
}
try:
    adapter.validate_python({"value": "incorrect wrapper"})
except ValidationError:
    pass
else:
    raise AssertionError("newtype must not accept an object wrapper")
print("Generated untagged newtypes preserve scalar and named-object wire shapes")
