"""JSON schemas of models whose custom hooks reshape the wire value.

The classes below follow the shape of generated code. Pydantic never runs a
model's validators or serializers to build its JSON schema, so each class
describes its wire shape through ``__get_pydantic_json_schema__``.
"""

# The classes spell unions as `Union[...]` and string enums as `(str, Enum)`,
# as generated code does.
# ruff: noqa: UP007, UP042

from __future__ import annotations

import json
from enum import Enum
from typing import Annotated, Any, Literal, Union

import jsonschema
import pytest
from pydantic import BaseModel, Field, RootModel, model_serializer, model_validator
from pydantic.json_schema import JsonSchemaMode

from reflectapi_runtime import (
    UnknownEnumValue,
    classify_tagged_enum,
    externally_tagged_json_schema,
    flattened_enum_json_schema,
    known_variants_json_schema,
    parse_externally_tagged,
    serialize_externally_tagged,
)

MODES: tuple[JsonSchemaMode, ...] = ("validation", "serialization")


class PairVariant(BaseModel):
    field_0: int
    field_1: str


class CountVariant(BaseModel):
    field_0: int


class NamedVariant(BaseModel):
    name: str


class ShapeUnknown(UnknownEnumValue):
    pass


class Shape(
    RootModel[
        Union[Literal["Empty"], PairVariant, CountVariant, NamedVariant, ShapeUnknown]
    ]
):
    """Externally tagged enum"""

    @model_validator(mode="before")
    @classmethod
    def _validate(cls, data: Any) -> Any:
        return parse_externally_tagged(
            data,
            {
                "Empty": "_unit",
                "Pair": lambda v: PairVariant(field_0=v[0], field_1=v[1]),
                "Count": lambda v: CountVariant(field_0=v),
                "Named": lambda v: NamedVariant.model_validate(v),
            },
            (PairVariant, CountVariant, NamedVariant),
            "Shape",
            unknown_type=ShapeUnknown,
        )

    @model_serializer
    def _serialize(self):  # a declared return type would become the schema
        return serialize_externally_tagged(
            self.root,
            {
                "Empty": (lambda r: r == "Empty", lambda r: "Empty"),
                "Pair": (
                    lambda r: isinstance(r, PairVariant),
                    lambda r: {"Pair": [r.field_0, r.field_1]},
                ),
                "Count": (
                    lambda r: isinstance(r, CountVariant),
                    lambda r: {"Count": r.field_0},
                ),
                "Named": (
                    lambda r: isinstance(r, NamedVariant),
                    lambda r: {"Named": r.model_dump(by_alias=True)},
                ),
            },
            "Shape",
        )

    @classmethod
    def __get_pydantic_json_schema__(cls, core_schema: Any, handler: Any) -> Any:
        return externally_tagged_json_schema(
            core_schema,
            handler,
            (
                ("Empty", "unit"),
                ("Pair", "tuple"),
                ("Count", "newtype"),
                ("Named", "struct"),
            ),
        )


class Dog(BaseModel):
    kind: Literal["Dog"] = "Dog"
    breed: str


class Bird(BaseModel):
    kind: Literal["Bird"] = "Bird"


class AnimalUnknown(UnknownEnumValue):
    pass


AnimalKnown = Annotated[Union[Dog, Bird], Field(discriminator="kind")]


class Animal(RootModel[Union[AnimalKnown, AnimalUnknown]]):
    root: Union[AnimalKnown, AnimalUnknown]

    @model_validator(mode="before")
    @classmethod
    def _classify(cls, data: Any) -> Any:
        return classify_tagged_enum(
            data,
            frozenset(("Dog", "Bird")),
            representation="tagged",
            tag_field="kind",
            unknown_type=AnimalUnknown,
        )

    @classmethod
    def __get_pydantic_json_schema__(cls, core_schema: Any, handler: Any) -> Any:
        return known_variants_json_schema(core_schema, handler)


class Envelope(BaseModel):
    id: int
    value: Animal

    @model_validator(mode="before")
    @classmethod
    def _unflatten_enum(cls, data: Any) -> Any:
        if not isinstance(data, dict) or "kind" not in data:
            return data
        parent = {key: value for key, value in data.items() if key == "id"}
        parent["value"] = {key: value for key, value in data.items() if key != "id"}
        return parent

    @model_serializer(mode="wrap")
    def _flatten_enum(self, handler: Any):  # likewise
        value = handler(self)
        enum_value = value.pop("value")
        return {**value, **enum_value}

    @classmethod
    def __get_pydantic_json_schema__(cls, core_schema: Any, handler: Any) -> Any:
        return flattened_enum_json_schema(core_schema, handler, "value", ("kind",))


class ClosedShape(RootModel[Union[Literal["Empty"], NamedVariant]]):
    @model_validator(mode="before")
    @classmethod
    def _validate(cls, data: Any) -> Any:
        return parse_externally_tagged(
            data,
            {"Empty": "_unit", "Named": lambda v: NamedVariant(**v)},
            (NamedVariant,),
            "ClosedShape",
        )

    @model_serializer
    def _serialize(self):  # a declared return type would become the schema
        return serialize_externally_tagged(
            self.root,
            {
                "Empty": (lambda r: r == "Empty", lambda r: "Empty"),
                "Named": (
                    lambda r: isinstance(r, NamedVariant),
                    lambda r: {"Named": r.model_dump(by_alias=True)},
                ),
            },
            "ClosedShape",
        )

    @classmethod
    def __get_pydantic_json_schema__(cls, core_schema: Any, handler: Any) -> Any:
        return externally_tagged_json_schema(
            core_schema, handler, (("Empty", "unit"), ("Named", "struct"))
        )


class OptionalEnvelope(BaseModel):
    id: int
    value: ClosedShape | None = None

    @model_validator(mode="before")
    @classmethod
    def _unflatten_enum(cls, data: Any) -> Any:
        if not isinstance(data, dict):
            return data
        present = {"Empty", "Named"}.intersection(data)
        parent = {key: value for key, value in data.items() if key == "id"}
        if present:
            key = next(iter(present))
            parent["value"] = {key: data[key]}
        return parent

    @model_serializer(mode="wrap")
    def _flatten_enum(self, handler: Any):  # likewise
        value = handler(self)
        enum_value = value.pop("value", None)
        if enum_value is None:
            return value
        if isinstance(enum_value, str):
            enum_value = {enum_value: None}
        return {**value, **enum_value}

    @classmethod
    def __get_pydantic_json_schema__(cls, core_schema: Any, handler: Any) -> Any:
        return flattened_enum_json_schema(
            core_schema, handler, "value", ("Empty", "Named"), optional=True
        )


def matches(schema: dict[str, Any], value: Any) -> bool:
    return jsonschema.Draft202012Validator(schema).is_valid(value)


WIRE = {
    Shape: ["Empty", {"Pair": [1, "one"]}, {"Count": 3}, {"Named": {"name": "n"}}],
    Animal: [{"kind": "Dog", "breed": "corgi"}, {"kind": "Bird"}],
    Envelope: [{"id": 1, "kind": "Dog", "breed": "corgi"}, {"id": 1, "kind": "Bird"}],
    OptionalEnvelope: [
        {"id": 1},
        {"id": 1, "Empty": None},
        {"id": 1, "Named": {"name": "n"}},
    ],
}


@pytest.mark.parametrize("mode", MODES)
@pytest.mark.parametrize(
    ("model", "wire"),
    [(model, wire) for model, wires in WIRE.items() for wire in wires],
    ids=lambda v: json.dumps(v) if not isinstance(v, type) else v.__name__,
)
def test_schema_accepts_the_wire_value(
    model: type[BaseModel], wire: Any, mode: JsonSchemaMode
) -> None:
    assert json.loads(model.model_validate(wire).model_dump_json()) == wire
    assert matches(model.model_json_schema(mode=mode), wire)


@pytest.mark.parametrize("mode", MODES)
@pytest.mark.parametrize(
    ("model", "wire"),
    [
        (Shape, {"field_0": 1, "field_1": "one"}),
        (Shape, {"Pair": [1]}),
        (Shape, {"Count": {"field_0": 3}}),
        (OptionalEnvelope, {"id": 1, "Empty": "Empty"}),
    ],
)
def test_schema_rejects_the_python_shape(
    model: type[BaseModel], wire: Any, mode: JsonSchemaMode
) -> None:
    assert not matches(model.model_json_schema(mode=mode), wire)


@pytest.mark.parametrize("mode", MODES)
def test_unknown_case_is_left_out(mode: JsonSchemaMode) -> None:
    for model in (Shape, Animal, Envelope):
        schema = json.dumps(model.model_json_schema(mode=mode))
        assert "Unknown" not in schema
        assert "x-reflectapi-unknown-variant" not in schema


def test_animal_keeps_discriminator() -> None:
    schema = Animal.model_json_schema()
    assert schema["discriminator"]["propertyName"] == "kind"
    assert len(schema["oneOf"]) == 2


@pytest.mark.parametrize("mode", MODES)
def test_flattened_enum_has_no_nested_field(mode: JsonSchemaMode) -> None:
    for model in (Envelope, OptionalEnvelope):
        schema = model.model_json_schema(mode=mode)
        enum, parent = schema["allOf"]
        assert parent["properties"].keys() == {"id"}
        assert parent["required"] == ["id"]


class Colour(str, Enum):
    RED = "Red"


class PaintUnknown(UnknownEnumValue):
    pass


# An open enum generated before `known_variants_json_schema`, without the hook
class Paint(RootModel[Union[Colour, PaintUnknown]]):
    root: Union[Colour, PaintUnknown]

    @model_validator(mode="before")
    @classmethod
    def _classify(cls, data: Any) -> Any:
        return classify_tagged_enum(
            data,
            frozenset(("Red",)),
            representation="external",
            unknown_type=PaintUnknown,
            unit_tags=frozenset(("Red",)),
        )


@pytest.mark.parametrize("mode", MODES)
def test_unknown_case_matches_nothing_without_the_enum_hook(
    mode: JsonSchemaMode,
) -> None:
    schema = Paint.model_json_schema(mode=mode)
    assert matches(schema, "Red")
    assert not matches(schema, "Blue")
    assert not matches(schema, {"tag": "Blue", "raw": "Blue"})
