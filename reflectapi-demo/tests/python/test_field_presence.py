"""Validate directional presence against SDKs generated from Rust fixtures."""

from __future__ import annotations

import asyncio
import json
import pathlib
import sys

import httpx
import pytest
from pydantic import TypeAdapter, ValidationError

FIXTURE_ROOT = pathlib.Path(__file__).resolve().parents[2] / "target/field-presence"
sys.path.insert(0, str(FIXTURE_ROOT / "python"))

from presence_client import AsyncClient, Client  # noqa: E402
from presence_client.field_presence.fixtures import Equal, Tuple, input, output  # noqa: E402

OUTPUT = {
    "plain": None,
    "custom": None,
    "defaulted": None,
    "serializer_only": None,
    "three_state": None,
    "default_three_state": None,
}


def test_request_omission_and_response_presence_are_directional() -> None:
    request = input.Fields.model_validate({"custom": None})
    assert request.model_fields_set == {"custom"}
    assert input.Fields.model_fields["custom"].is_required()
    assert not input.Fields.model_fields["plain"].is_required()
    assert not input.Fields.model_fields["serializer_only"].is_required()
    with pytest.raises(ValidationError):
        input.Fields.model_validate({})
    assert output.Fields.model_validate(OUTPUT).plain is None
    for required in OUTPUT:
        with pytest.raises(ValidationError):
            output.Fields.model_validate(
                {k: v for k, v in OUTPUT.items() if k != required}
            )
    assert not output.Fields.model_fields["skipped"].is_required()


def test_struct_variants_flatten_and_option_type_parameters() -> None:
    TypeAdapter(input.Variant).validate_python({"kind": "Value"})
    with pytest.raises(ValidationError):
        TypeAdapter(output.Variant).validate_python({"kind": "Value"})
    input.Flat.model_validate({"id": 1, "custom": None})
    output.Flat.model_validate({"id": 1, **OUTPUT})
    with pytest.raises(ValidationError):
        output.Flat.model_validate({"id": 1, "custom": None})
    input.Wrapper[str].model_validate({})
    with pytest.raises(ValidationError):
        output.Wrapper[str].model_validate({})
    assert output.Wrapper[str].model_validate({"value": None}).value is None
    Tuple.model_validate([None, None])
    with pytest.raises(ValidationError):
        Tuple.model_validate([])
    Equal.model_validate({})


@pytest.mark.parametrize("model", [input.OptionalFlat, output.OptionalFlat])
def test_optional_flattened_group_is_all_or_none(model: type) -> None:
    assert model.model_validate({"id": 1}).model_dump(exclude_unset=True) == {"id": 1}
    model.model_validate({"id": 1, "left": 2, "right": 3})
    for partial in ({"left": 2}, {"right": 3}):
        with pytest.raises(ValidationError):
            model.model_validate({"id": 1, **partial})


@pytest.mark.parametrize("asynchronous", [False, True])
@pytest.mark.parametrize("explicit_null", [False, True])
@pytest.mark.parametrize("flattened", [False, True])
def test_sdk_transport_retains_only_explicit_request_fields(
    asynchronous: bool, explicit_null: bool, flattened: bool
) -> None:
    supplied = {"custom": None}
    if explicit_null:
        supplied.update(OUTPUT, skipped=None)
    wire = {"id": 1, **supplied} if flattened else supplied
    model = (
        input.Flat.model_validate(wire)
        if flattened
        else input.Fields.model_validate(wire)
    )
    response_wire = {"id": 1, **OUTPUT} if flattened else OUTPUT
    seen = []

    def handle(request: httpx.Request) -> httpx.Response:
        seen.append(json.loads(request.content))
        return httpx.Response(200, json=response_wire)

    if asynchronous:

        async def run() -> None:
            async with AsyncClient(
                "http://test",
                client=httpx.AsyncClient(transport=httpx.MockTransport(handle)),
            ) as client:
                call = client.fields.flat if flattened else client.fields.echo
                await call(model)

        asyncio.run(run())
    else:
        with Client(
            "http://test", client=httpx.Client(transport=httpx.MockTransport(handle))
        ) as client:
            call = client.fields.flat if flattened else client.fields.echo
            call(model)
    assert seen == [wire]


@pytest.mark.parametrize("asynchronous", [False, True])
@pytest.mark.parametrize("explicit_null", [False, True])
def test_nested_ordinary_option_models_preserve_omission(
    asynchronous: bool, explicit_null: bool
) -> None:
    # No reflectapi::Option fields: these classes inherit ordinary BaseModel,
    # so omission depends on the SDK request serializer's recursive behavior.
    child = {"value": None} if explicit_null else {}
    wire = {"one": child, "many": [child]}
    model = input.Nested.model_validate(wire)
    seen = []

    def handle(request: httpx.Request) -> httpx.Response:
        seen.append(json.loads(request.content))
        return httpx.Response(
            200, json={"one": {"value": None}, "many": [{"value": None}]}
        )

    if asynchronous:

        async def run() -> None:
            async with AsyncClient(
                "http://test",
                client=httpx.AsyncClient(transport=httpx.MockTransport(handle)),
            ) as client:
                await client.fields.nested(model)

        asyncio.run(run())
    else:
        with Client(
            "http://test", client=httpx.Client(transport=httpx.MockTransport(handle))
        ) as client:
            client.fields.nested(model)
    assert seen == [wire]


@pytest.mark.parametrize("asynchronous", [False, True])
def test_defaulted_non_nullable_fields_allow_only_omission(asynchronous: bool) -> None:
    model = input.Defaults.model_validate({})
    with pytest.raises(ValidationError):
        input.Defaults.model_validate({"count": None})
    with pytest.raises(ValidationError):
        model.count = None
    with pytest.raises(ValidationError):
        output.Defaults.model_validate({})
    assert output.Defaults.model_validate({"count": 0}).count == 0
    seen = []

    def handle(request: httpx.Request) -> httpx.Response:
        seen.append(json.loads(request.content))
        return httpx.Response(200, json={"count": 0})

    if asynchronous:

        async def run() -> None:
            async with AsyncClient(
                "http://test",
                client=httpx.AsyncClient(transport=httpx.MockTransport(handle)),
            ) as client:
                await client.fields.defaults(model)

        asyncio.run(run())
    else:
        with Client(
            "http://test", client=httpx.Client(transport=httpx.MockTransport(handle))
        ) as client:
            client.fields.defaults(model)
    assert seen == [{}]


@pytest.mark.parametrize("asynchronous", [False, True])
def test_defaulted_discriminator_is_always_sent(asynchronous: bool) -> None:
    variant = input.VariantValue(value=None)
    wire = {"kind": "Value", "value": None}
    assert json.loads(variant.model_dump_json()) == wire
    assert json.loads(variant.model_dump_json(exclude_unset=True)) == wire
    model = input.Variant(root=variant)
    seen = []

    def handle(request: httpx.Request) -> httpx.Response:
        seen.append(json.loads(request.content))
        return httpx.Response(200, json=wire)

    if asynchronous:

        async def run() -> None:
            async with AsyncClient(
                "http://test",
                client=httpx.AsyncClient(transport=httpx.MockTransport(handle)),
            ) as client:
                await client.fields.variant(model)

        asyncio.run(run())
    else:
        with Client(
            "http://test", client=httpx.Client(transport=httpx.MockTransport(handle))
        ) as client:
            client.fields.variant(model)
    assert seen == [wire]


def test_direct_model_dumps_preserve_omission_and_explicit_null() -> None:
    defaulted = input.Defaults()
    assert defaulted.model_dump() == {}
    assert input.Defaults.model_validate_json(defaulted.model_dump_json()).count is None
    assert input.Wrapper[str]().model_dump() == {}
    assert input.Wrapper[str](value=None).model_dump() == {"value": None}
