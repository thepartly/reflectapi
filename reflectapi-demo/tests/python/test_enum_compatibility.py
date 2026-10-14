"""Replay newer enum wire values through a client generated from an older schema."""

from __future__ import annotations

import asyncio
import json
import pathlib
import sys

import httpx
import pytest
from pydantic import TypeAdapter, ValidationError
from reflectapi_runtime import (
    ApplicationError,
    ValidationError as ResponseValidationError,
)

FIXTURE_ROOT = pathlib.Path(__file__).resolve().parents[2] / "target/enum-compatibility"
sys.path.insert(0, str(FIXTURE_ROOT / "python"))

from enum_client import AsyncClient, Client  # noqa: E402
from enum_client.enum_compatibility.fixtures import (  # noqa: E402
    ClosedExternal,
    ClosedExternalValueVariant,
    input,
    output,
)


def dumped(value: object) -> object:
    return json.loads(value.model_dump_json(by_alias=True))


@pytest.mark.parametrize(
    "representation", ["internal", "adjacent", "flatten_internal", "flatten_adjacent"]
)
def test_serde_other_matches_rust_oracle(representation: str) -> None:
    oracle = json.loads((FIXTURE_ROOT / "fallback_samples.json").read_text())
    model = {
        "internal": output.FallbackInternal,
        "adjacent": output.FallbackAdjacent,
        "flatten_internal": output.FlattenFallbackEnvelope,
        "flatten_adjacent": output.FlattenAdjacentFallbackEnvelope,
    }[representation]
    for sample in oracle[representation]:
        if sample["ok"]:
            assert dumped(model.model_validate(sample["wire"])) == sample["value"]
        else:
            with pytest.raises(ValidationError):
                model.model_validate(sample["wire"])


@pytest.mark.parametrize(
    ("model", "wire", "unknown_class"),
    [
        (
            output.OpenInternal,
            {"kind": "Fox", "tail": {"length": 3}},
            output.OpenInternalUnknown,
        ),
        (
            output.OpenAdjacent,
            {"kind": "Future", "data": [1, 2]},
            output.OpenAdjacentUnknown,
        ),
        (output.OpenExternal, {"Future": {"nested": [1]}}, output.OpenExternalUnknown),
        (output.OpenExternal, "Future", output.OpenExternalUnknown),
        (output.OpenUnit, "Future", output.OpenUnitUnknown2),
        (output.OpenUnit, {"Future": None}, output.OpenUnitUnknown2),
    ],
)
def test_open_cases_keep_tag_and_raw_wire(model, wire, unknown_class) -> None:
    result = model.model_validate(wire)
    assert isinstance(result.root, unknown_class)
    assert result.root.tag == (
        wire if isinstance(wire, str) else wire.get("kind", next(iter(wire)))
    )
    assert result.root.raw == wire
    assert dumped(result) == wire


def test_known_cases_remain_typed_and_nested_unknowns_round_trip() -> None:
    wire = {
        "internal": {"kind": "Fox", "tail": 2},
        "adjacent": {"kind": "Unit"},
        "external": {"Value": {"message": "known"}},
        "units": "Ready",
        "generic": {"kind": "Future", "value": [1]},
        "fallback": {"kind": "Later", "ignored": True},
        "adjacent_fallback": {"kind": "Later", "data": None},
        "optional": {"kind": "Future", "optional": True},
        "list": [{"kind": "Future", "index": 1}],
        "map": {"x": {"kind": "Future", "index": 2}},
        "alias": {"kind": "Future", "alias": True},
    }
    envelope = output.Envelope.model_validate(wire)
    assert isinstance(envelope.internal.root, output.OpenInternalUnknown)
    assert isinstance(envelope.adjacent.root, output.OpenAdjacentUnit)
    assert isinstance(envelope.external.root, output.OpenExternalValueVariant)
    assert isinstance(envelope.generic.root, output.OpenGenericUnknown)
    assert isinstance(envelope.list_[0].root, output.OpenInternalUnknown)
    assert isinstance(envelope.map_["x"].root, output.OpenInternalUnknown)
    assert dumped(envelope) == {
        **wire,
        "fallback": {"kind": "other"},
        "adjacent_fallback": {"kind": "Other"},
    }


def test_unknowns_survive_ordinary_and_flattened_fields() -> None:
    wire = {
        "ordinary": "FutureOrdinary",
        "first": "FutureFlattenedUnit",
        "second": {"kind": "FutureFlattenedTagged", "payload": {"new": True}},
    }
    envelope = output.FlattenEnvelope.model_validate(wire)
    assert isinstance(envelope.ordinary.root, output.OpenUnitUnknown2)
    assert envelope.ordinary.root.tag == "FutureOrdinary"
    assert isinstance(envelope.first.root, output.OpenUnitUnknown2)
    assert envelope.first.root.raw == "FutureFlattenedUnit"
    assert isinstance(envelope.second.root, output.OpenInternalUnknown)
    assert envelope.second.root.raw == wire["second"]
    assert dumped(envelope) == wire


def test_open_enum_inside_untagged_wrapper_retains_unknown() -> None:
    inner = {"kind": "FutureNested", "payload": [1, 2]}
    wrapped = TypeAdapter(output.UntaggedWrapper).validate_python(inner)
    assert isinstance(wrapped, output.UntaggedWrapperTagged)
    assert isinstance(wrapped.root.root, output.OpenInternalUnknown)
    assert wrapped.root.root.tag == "FutureNested"
    assert wrapped.root.root.raw == inner

    wire = {"kind": "Wrapped", "value": inner}
    outer = output.OpenNested.model_validate(wire)
    assert isinstance(outer.root, output.OpenNestedWrapped)
    assert isinstance(outer.root.value.root.root, output.OpenInternalUnknown)
    assert dumped(outer) == wire


def test_malformed_known_enum_inside_untagged_wrapper_rejects() -> None:
    bad_inner = {"kind": "Dog", "breed": 8}
    with pytest.raises(ValidationError):
        TypeAdapter(output.UntaggedWrapper).validate_python(bad_inner)
    with pytest.raises(ValidationError):
        output.OpenNested.model_validate({"kind": "Wrapped", "value": bad_inner})


def test_generated_routes_parse_flattened_and_nested_unknowns() -> None:
    flattened = {
        "ordinary": "FutureOrdinary",
        "first": "FutureFlattenedUnit",
        "second": {"kind": "FutureFlattenedTagged", "new": True},
    }
    nested = {"kind": "Wrapped", "value": {"kind": "FutureNested", "new": True}}

    def handle(request: httpx.Request) -> httpx.Response:
        body = flattened if request.url.path == "/enums.flatten" else nested
        return httpx.Response(200, json=body)

    client = Client(
        "http://test", client=httpx.Client(transport=httpx.MockTransport(handle))
    )
    assert dumped(client.enums.flatten().data) == flattened
    assert dumped(client.enums.nested().data) == nested
    client.close()


def test_direct_flattened_open_enum_retains_parent_and_unknown_value() -> None:
    wire = {"id": 7, "kind": "FutureFlattened", "new": {"nested": True}}
    envelope = output.FlattenEnumEnvelope.model_validate(wire)
    assert envelope.id == 7
    assert isinstance(envelope.value.root, output.OpenInternalUnknown)
    assert envelope.value.root.tag == "FutureFlattened"
    assert envelope.value.root.raw == {
        "kind": "FutureFlattened",
        "new": {"nested": True},
    }
    assert dumped(envelope) == wire

    def handle(_request: httpx.Request) -> httpx.Response:
        return httpx.Response(200, json=wire)

    client = Client(
        "http://test", client=httpx.Client(transport=httpx.MockTransport(handle))
    )
    assert dumped(client.enums_flatten.enum().data) == wire
    client.close()


def test_direct_flattened_open_enum_known_case_stays_typed() -> None:
    wire = {"id": 8, "kind": "Dog", "breed": "collie"}
    envelope = output.FlattenEnumEnvelope.model_validate(wire)
    assert envelope.id == 8
    assert isinstance(envelope.value.root, output.OpenInternalDog)
    assert envelope.value.root.breed == "collie"
    assert dumped(envelope) == wire
    assert dumped(input.FlattenEnumEnvelope.model_validate(wire)) == wire


def test_direct_flattened_open_enum_rejects_malformed_known_and_closed_input() -> None:
    with pytest.raises(ValidationError):
        output.FlattenEnumEnvelope.model_validate({"id": 9, "kind": "Dog", "breed": 8})
    with pytest.raises(ValidationError):
        output.FlattenEnumEnvelope.model_validate(
            {"id": 9, "value": {"kind": "Future"}}
        )
    with pytest.raises(ValidationError):
        output.FlattenEnumEnvelope.model_validate({"id": 9})
    with pytest.raises(ValidationError):
        input.FlattenEnumEnvelope.model_validate({"id": 9, "kind": "Future"})


def test_optional_direct_flattened_enum_missing_and_unknown_values() -> None:
    absent = output.OptionalFlattenEnumEnvelope.model_validate({"id": 7})
    assert absent.value is None
    assert dumped(absent) == {"id": 7}

    ignored = output.OptionalFlattenEnumEnvelope.model_validate(
        {"id": 7, "unrelated": 8}
    )
    assert ignored.value is None
    assert dumped(ignored) == {"id": 7}

    wire = {"id": 7, "kind": "Future", "extra": {"new": True}}
    unknown = output.OptionalFlattenEnumEnvelope.model_validate(wire)
    assert isinstance(unknown.value.root, output.OpenInternalUnknown)
    assert unknown.value.root.raw == {"kind": "Future", "extra": {"new": True}}
    assert dumped(unknown) == wire

    known_wire = {"id": 8, "kind": "Dog", "breed": "collie"}
    known = output.OptionalFlattenEnumEnvelope.model_validate(known_wire)
    assert isinstance(known.value.root, output.OpenInternalDog)
    assert dumped(known) == known_wire

    with pytest.raises(ValidationError):
        output.OptionalFlattenEnumEnvelope.model_validate(
            {"id": 7, "value": {"kind": "Future"}}
        )
    with pytest.raises(ValidationError):
        output.OptionalFlattenEnumEnvelope.model_validate(
            {"id": 7, "kind": "Dog", "breed": 8}
        )


def test_direct_flattened_enum_accepts_typed_sdk_construction() -> None:
    value = output.OpenInternal.model_validate({"kind": "Future", "new": True})
    required = output.FlattenEnumEnvelope(id=7, value=value)
    optional = output.OptionalFlattenEnumEnvelope(id=7, value=value)
    assert dumped(required) == {"id": 7, "kind": "Future", "new": True}
    assert dumped(optional) == {"id": 7, "kind": "Future", "new": True}


def test_optional_direct_flattened_enum_response_path() -> None:
    wire = {"id": 7, "kind": "Future", "new": True}

    def handle(_request: httpx.Request) -> httpx.Response:
        return httpx.Response(200, json=wire)

    client = Client(
        "http://test", client=httpx.Client(transport=httpx.MockTransport(handle))
    )
    response = client.enums_optional.flatten_enum()
    assert isinstance(response.data.value.root, output.OpenInternalUnknown)
    assert dumped(response.data) == wire
    client.close()


def test_flattened_open_enum_keeps_parent_tag_and_raw_fields_separate() -> None:
    wire = {
        "id": 7,
        "tag": "parent tag",
        "raw": "parent raw",
        "kind": "Future",
        "payload": {"new": True},
    }
    envelope = output.FlattenNamedEnvelope.model_validate(wire)
    assert envelope.tag == "parent tag"
    assert envelope.raw == "parent raw"
    assert isinstance(envelope.value.root, output.OpenInternalUnknown)
    assert envelope.value.root.tag == "Future"
    assert envelope.value.root.raw == {"kind": "Future", "payload": {"new": True}}
    assert dumped(envelope) == wire


def test_open_enum_nested_through_flattened_struct_round_trips() -> None:
    wire = {"ordinary": "parent", "id": 7, "kind": "Future", "new": {"x": 1}}
    envelope = output.NestedFlattenEnvelope.model_validate(wire)
    assert envelope.ordinary == "parent"
    assert envelope.id == 7
    assert isinstance(envelope.value.root, output.OpenInternalUnknown)
    assert envelope.value.root.raw == {"kind": "Future", "new": {"x": 1}}
    assert dumped(envelope) == wire

    known_wire = {"ordinary": "parent", "id": 7, "kind": "Dog", "breed": "collie"}
    known = output.NestedFlattenEnvelope.model_validate(known_wire)
    assert isinstance(known.value.root, output.OpenInternalDog)
    assert dumped(known) == known_wire


def test_flattened_closed_external_enum_selects_known_variant_key() -> None:
    ordinary = {"kind": "Future", "new": 1}
    wire = {"id": 7, "ordinary": ordinary, "Value": {"message": "known"}}
    parsed = output.FlattenClosedExternalEnvelope.model_validate(wire)
    assert isinstance(parsed.ordinary.root, output.OpenInternalUnknown)
    assert isinstance(parsed.value.root, ClosedExternalValueVariant)
    assert dumped(parsed) == wire

    built = output.FlattenClosedExternalEnvelope.model_validate(
        {
            "id": 7,
            "ordinary": parsed.ordinary,
            "value": ClosedExternal.model_validate({"Value": {"message": "known"}}),
        }
    )
    assert dumped(built) == wire

    absent = output.OptionalFlattenClosedExternalEnvelope.model_validate(
        {"id": 7, "ordinary": ordinary}
    )
    assert absent.value is None
    assert dumped(absent) == {"id": 7, "ordinary": ordinary}

    with pytest.raises(ValidationError):
        output.FlattenClosedExternalEnvelope.model_validate(
            {"id": 7, "ordinary": ordinary, "Future": {"message": "new"}}
        )


def test_direct_flattened_adjacent_open_enum_and_optional_field() -> None:
    known_wire = {"id": 7, "kind": "Value", "data": {"message": "known"}}
    known = output.FlattenAdjacentEnvelope.model_validate(known_wire)
    assert isinstance(known.value.root, output.OpenAdjacentValue)
    assert dumped(known) == known_wire

    wire = {"id": 7, "kind": "Future", "data": {"new": [1, 2]}}
    unknown = output.FlattenAdjacentEnvelope.model_validate(wire)
    assert isinstance(unknown.value.root, output.OpenAdjacentUnknown)
    assert unknown.value.root.tag == "Future"
    assert unknown.value.root.raw == {"kind": "Future", "data": {"new": [1, 2]}}
    assert dumped(unknown) == wire

    absent = output.OptionalFlattenAdjacentEnvelope.model_validate({"id": 7})
    assert absent.value is None
    assert dumped(absent) == {"id": 7}
    optional = output.OptionalFlattenAdjacentEnvelope.model_validate(wire)
    assert isinstance(optional.value.root, output.OpenAdjacentUnknown)
    assert dumped(optional) == wire

    with pytest.raises(ValidationError):
        output.FlattenAdjacentEnvelope.model_validate(
            {"id": 7, "kind": "Value", "data": {}}
        )

    def handle(_request: httpx.Request) -> httpx.Response:
        return httpx.Response(200, json=wire)

    client = Client(
        "http://test", client=httpx.Client(transport=httpx.MockTransport(handle))
    )
    assert dumped(client.enums_flatten.adjacent().data) == wire
    assert dumped(client.enums_optional.flatten_adjacent().data) == wire
    client.close()


def test_direct_flattened_declared_fallbacks_normalize_unknown_tags() -> None:
    internal = output.FlattenFallbackEnvelope.model_validate(
        {"id": 7, "kind": "Future", "new": True}
    )
    assert isinstance(internal.value.root, output.FallbackInternalOther)
    assert dumped(internal) == {"id": 7, "kind": "other"}

    adjacent = output.FlattenAdjacentFallbackEnvelope.model_validate(
        {"id": 7, "kind": "Future", "data": None}
    )
    assert isinstance(adjacent.value.root, output.FallbackAdjacentOther)
    assert dumped(adjacent) == {"id": 7, "kind": "Other"}


@pytest.mark.parametrize(
    ("model", "wire"),
    [
        (output.OpenInternal, {"kind": "Dog", "breed": 8}),
        (output.OpenInternal, {"kind": "Dog"}),
        (output.OpenInternal, {"breed": "collie"}),
        (output.OpenInternal, {"kind": 8}),
        (output.OpenAdjacent, {"kind": "Unit", "data": {"bad": True}}),
        (output.OpenAdjacent, {"kind": "Value", "data": {}}),
        (output.OpenExternal, {"Unit": {"bad": True}}),
        (output.OpenExternal, {"Unit": 1}),
        (output.OpenExternal, {"Value": [1]}),
        (output.OpenExternal, {"Value": None}),
        (output.OpenUnit, {"Ready": {"bad": True}}),
        (output.OpenUnit, {"Ready": 1}),
        (output.OpenExternal, {"Tuple": [1]}),
        (output.OpenExternal, {"Tuple": [1, "ok", 3]}),
    ],
)
def test_malformed_known_cases_do_not_use_unknown(model, wire) -> None:
    with pytest.raises((ValidationError, ValueError)):
        model.model_validate(wire)


def test_external_unit_object_with_null_payload_is_valid() -> None:
    assert (
        output.OpenUnit.model_validate({"Ready": None}).root
        == output.OpenUnitKnown.READY
    )
    assert output.OpenExternal.model_validate({"Unit": None}).root == "Unit"


def test_malformed_known_external_response_raises_sdk_validation_error() -> None:
    def handle(_request: httpx.Request) -> httpx.Response:
        return httpx.Response(200, json={"Value": [1]})

    client = Client(
        "http://test", client=httpx.Client(transport=httpx.MockTransport(handle))
    )
    with pytest.raises(ResponseValidationError):
        client.enums.external()
    client.close()


def test_request_enums_stay_closed() -> None:
    with pytest.raises(ValidationError):
        input.OpenInternal.model_validate({"kind": "Future"})
    with pytest.raises(ValidationError):
        input.OpenExternal.model_validate({"Future": {"x": 1}})


def test_sync_and_async_response_paths() -> None:
    wire = {"kind": "Future", "nested": {"x": 1}}

    def handle(_request: httpx.Request) -> httpx.Response:
        return httpx.Response(200, json=wire)

    sync = Client(
        "http://test", client=httpx.Client(transport=httpx.MockTransport(handle))
    )
    assert isinstance(sync.enums.internal().data.root, output.OpenInternalUnknown)
    assert dumped(sync.enums.internal().data) == wire
    sync.close()

    async def check_async() -> None:
        client = AsyncClient(
            "http://test",
            client=httpx.AsyncClient(transport=httpx.MockTransport(handle)),
        )
        response = await client.enums.internal()
        assert isinstance(response.data.root, output.OpenInternalUnknown)
        assert dumped(response.data) == wire
        await client.aclose()

    asyncio.run(check_async())


@pytest.mark.parametrize("header", [None, "Ready", "Future"])
def test_declared_response_header_uses_open_unit_enum(header: str | None) -> None:
    def handle(_request: httpx.Request) -> httpx.Response:
        headers = {"x-kind": header} if header is not None else {}
        return httpx.Response(200, json={"kind": "Bird"}, headers=headers)

    client = Client(
        "http://test", client=httpx.Client(transport=httpx.MockTransport(handle))
    )
    parsed = client.enums.internal().headers.x_kind
    if header is None:
        assert parsed is None
    elif header == "Ready":
        assert parsed.root == output.OpenUnitKnown.READY
    else:
        assert isinstance(parsed.root, output.OpenUnitUnknown2)
        assert parsed.root.tag == "Future"
        assert parsed.root.raw == "Future"
    client.close()


def test_typed_error_and_sse_items_use_open_model() -> None:
    wire = {"kind": "Future", "event": 1}

    def handle(request: httpx.Request) -> httpx.Response:
        if request.url.path == "/enums.error":
            return httpx.Response(400, json=wire)
        body = f"data: {json.dumps(wire)}\n\n".encode()
        return httpx.Response(200, content=body)

    client = Client(
        "http://test", client=httpx.Client(transport=httpx.MockTransport(handle))
    )
    with pytest.raises(ApplicationError) as error:
        client.enums.error()
    assert isinstance(error.value.typed_error.root, output.OpenInternalUnknown)
    assert dumped(error.value.typed_error) == wire
    stream_items = list(client.enums.stream())
    assert len(stream_items) == 1
    assert isinstance(stream_items[0].root, output.OpenInternalUnknown)
    assert dumped(stream_items[0]) == wire
    client.close()
