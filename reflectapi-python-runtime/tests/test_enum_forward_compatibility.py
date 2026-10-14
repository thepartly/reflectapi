"""The runtime must preserve future response cases without hiding bad known cases."""

from __future__ import annotations

from typing import Literal

import pytest
from pydantic import BaseModel, RootModel, ValidationError, model_validator

from reflectapi_runtime import (
    UnknownEnumValue,
    classify_tagged_enum,
    parse_externally_tagged,
    serialize_externally_tagged,
)


class Known(BaseModel):
    kind: Literal["Known"]
    value: int


class FutureUnknown(UnknownEnumValue):
    pass


class OpenResult(RootModel[Known | FutureUnknown]):
    root: Known | FutureUnknown

    @model_validator(mode="before")
    @classmethod
    def _classify(cls, data):
        return classify_tagged_enum(
            data,
            frozenset(("Known",)),
            representation="tagged",
            tag_field="kind",
            unknown_type=FutureUnknown,
        )


class Envelope(BaseModel):
    results: list[OpenResult]


def test_open_case_round_trips_inside_nested_model():
    wire = {"results": [{"kind": "Known", "value": 2}, {"kind": "Future", "x": [1]}]}
    parsed = Envelope.model_validate(wire)
    assert isinstance(parsed.results[0].root, Known)
    unknown = parsed.results[1].root
    assert isinstance(unknown, FutureUnknown)
    assert unknown.tag == "Future"
    assert unknown.raw == wire["results"][1]
    assert parsed.model_dump() == wire
    assert Envelope.model_validate_json(parsed.model_dump_json()).model_dump() == wire


@pytest.mark.parametrize(
    "value",
    [
        {"kind": "Known", "value": "not an integer"},
        {"value": 2},
        {"kind": 42, "value": 2},
        {"kind": None, "value": 2},
    ],
)
def test_open_case_rejects_malformed_known_or_missing_tag(value):
    with pytest.raises(ValidationError):
        OpenResult.model_validate(value)


def test_other_fallback_discards_unfamiliar_payload():
    assert classify_tagged_enum(
        {"kind": "Future", "payload": {"x": 1}},
        frozenset(("Known", "Other")),
        representation="tagged",
        tag_field="kind",
        fallback_tag="Other",
    ) == {"kind": "Other"}
    assert classify_tagged_enum(
        {"kind": "Known", "value": "bad"},
        frozenset(("Known", "Other")),
        representation="tagged",
        tag_field="kind",
        fallback_tag="Other",
    ) == {"kind": "Known", "value": "bad"}


def test_external_unknown_string_and_single_key_object():
    for wire in ("Future", {"Future": {"x": 1}}):
        unknown = parse_externally_tagged(
            wire,
            {"Known": "_unit"},
            (),
            "Result",
            unknown_type=FutureUnknown,
        )
        assert isinstance(unknown, FutureUnknown)
        assert unknown.tag == "Future"
        assert serialize_externally_tagged(unknown, {}, "Result") == wire


def test_external_unit_object_normalizes_to_known_literal():
    assert classify_tagged_enum(
        {"Known": None},
        frozenset(("Known",)),
        representation="external",
        unit_tags=frozenset(("Known",)),
    ) == "Known"
    for payload in (1, {"bad": True}):
        with pytest.raises(ValueError):
            classify_tagged_enum(
                {"Known": payload},
                frozenset(("Known",)),
                representation="external",
                unit_tags=frozenset(("Known",)),
            )


@pytest.mark.parametrize("wire", [{"A": 1, "B": 2}, {42: "value"}, None, 42])
def test_external_malformed_tags_do_not_become_unknown(wire):
    with pytest.raises(ValueError):
        parse_externally_tagged(
            wire,
            {"Known": "_unit"},
            (),
            "Result",
            unknown_type=FutureUnknown,
        )
