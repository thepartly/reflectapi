"""Check generated models' JSON schemas against serde's wire values.

Pydantic builds a JSON schema from a model's fields and never runs its custom
validators or serializers, so a model that reshapes the wire in those hooks
must describe the wire shape itself. A FastAPI app that returns generated
models publishes these schemas as its OpenAPI spec.

`cargo test -p reflectapi-demo --test enum_compatibility` writes the client and
`known_samples.json`: serde's output for every known variant of each fixture.
Each value must round-trip through the generated model, and both the
validation-mode schema (what the model accepts) and the serialization-mode
schema (what it emits) must accept it.
"""

from __future__ import annotations

import json
import pathlib
import sys
from typing import Any

import jsonschema
import pytest
from pydantic import TypeAdapter
from pydantic.json_schema import JsonSchemaMode

FIXTURE_ROOT = pathlib.Path(__file__).resolve().parents[2] / "target/enum-compatibility"
sys.path.insert(0, str(FIXTURE_ROOT / "python"))

from enum_client.enum_compatibility import fixtures  # noqa: E402
from enum_client.enum_compatibility.fixtures import output  # noqa: E402

SAMPLES: dict[str, list[Any]] = json.loads(
    (FIXTURE_ROOT / "known_samples.json").read_text()
)
# Output models are the ones that carry open and fallback variants, and the
# ones a server returns.
CASES = [
    pytest.param(name, wire, id=f"{name}-{i}")
    for name, wires in SAMPLES.items()
    for i, wire in enumerate(wires)
]
MODES: tuple[JsonSchemaMode, ...] = ("validation", "serialization")


def adapter(name: str) -> TypeAdapter[Any]:
    return TypeAdapter(getattr(output, name, None) or getattr(fixtures, name))


@pytest.mark.parametrize(("name", "wire"), CASES)
@pytest.mark.parametrize("mode", MODES)
def test_schema_describes_serde_wire(
    name: str, wire: Any, mode: JsonSchemaMode
) -> None:
    model = adapter(name)
    # The sample is a real wire value for this model, not just a schema match.
    assert (
        model.dump_python(model.validate_python(wire), mode="json", by_alias=True)
        == wire
    )

    schema = model.json_schema(mode=mode)
    jsonschema.Draft202012Validator.check_schema(schema)
    jsonschema.validate(wire, schema, cls=jsonschema.Draft202012Validator)


def test_every_fixture_has_known_samples() -> None:
    for name, wires in SAMPLES.items():
        assert wires, name
        assert hasattr(output, name) or hasattr(fixtures, name), name


@pytest.mark.parametrize("mode", MODES)
def test_externally_tagged_schema_matches_wire_shape(mode: JsonSchemaMode) -> None:
    schema = TypeAdapter(output.OpenExternal).json_schema(mode=mode)
    variants = schema["oneOf"]
    assert variants[0] == {"const": "Unit", "type": "string"}
    by_tag = {next(iter(v["properties"])): v for v in variants[1:]}
    assert set(by_tag) == {"Value", "Number", "Tuple", "Optional"}
    for tag, variant in by_tag.items():
        assert variant["required"] == [tag]
    assert by_tag["Number"]["properties"]["Number"] == {"type": "integer"}
    assert by_tag["Tuple"]["properties"]["Tuple"] == {
        "type": "array",
        "prefixItems": [{"type": "integer"}, {"type": "string"}],
        "minItems": 2,
        "maxItems": 2,
    }
    value = by_tag["Value"]["properties"]["Value"]
    assert value["$ref"].endswith("OpenExternalValueVariant")
    # The unknown case of an open enum is not part of the described wire.
    assert not any(key.endswith("Unknown") for key in schema.get("$defs", {}))


@pytest.mark.parametrize("mode", MODES)
def test_flattened_enum_schema_has_no_nested_value(mode: JsonSchemaMode) -> None:
    schema = TypeAdapter(output.FlattenEnumEnvelope).json_schema(mode=mode)
    enum, parent = schema["allOf"]
    assert parent["properties"].keys() == {"id"}
    assert parent["required"] == ["id"]
    assert enum["$ref"].endswith("OpenInternal")


@pytest.mark.parametrize("mode", MODES)
def test_optional_flattened_enum_may_be_absent(mode: JsonSchemaMode) -> None:
    schema = TypeAdapter(output.OptionalFlattenEnumEnvelope).json_schema(mode=mode)
    for wire in ({"id": 7}, {"id": 7, "kind": "Bird"}):
        jsonschema.validate(wire, schema)
    with pytest.raises(jsonschema.ValidationError):
        jsonschema.validate({"id": 7, "kind": "Dog"}, schema)
