"""Static contract exercised by a consumer of the generated enum SDK.

Run with the adjacent strict pyrightconfig.json after generating the fixture.
These functions are checked, not called: no server or transport is needed.
"""

from __future__ import annotations

from typing import Any, assert_type

from enum_client import AsyncClient, Client
from enum_client.enum_compatibility.fixtures import (
    ClosedExternal,
    ResponseHeaders,
    input,
    output,
)
from reflectapi_runtime import ApiResponse


def check_internal_variants(value: output.OpenInternal) -> None:
    variant = value.root
    if isinstance(variant, output.OpenInternalDog):
        assert_type(variant, output.OpenInternalDog)
        assert_type(variant.breed, str)
    elif isinstance(variant, output.OpenInternalUnknown):
        assert_type(variant, output.OpenInternalUnknown)
        assert_type(variant.tag, str)
        assert_type(variant.raw, Any)


def check_generic_known_variant(value: output.OpenGeneric[str]) -> None:
    variant = value.root
    if isinstance(variant, output.OpenGenericValue):
        assert_type(variant.value, str)
    elif isinstance(variant, output.OpenGenericUnknown):
        assert_type(variant.tag, str)
        assert_type(variant.raw, Any)


def check_closed_request(unknown: output.OpenInternalUnknown) -> None:
    input.OpenInternalUnknown  # pyright: ignore[reportUnknownMemberType, reportAttributeAccessIssue]
    input.OpenInternal(root=unknown)  # pyright: ignore[reportArgumentType]


def check_optional_flatten(value: output.OptionalFlattenEnumEnvelope) -> None:
    assert_type(value.value, output.OpenInternal | None)
    if value.value is None:
        assert_type(value.value, None)
    else:
        assert_type(value.value, output.OpenInternal)


def check_optional_adjacent_flatten(
    value: output.OptionalFlattenAdjacentEnvelope,
) -> None:
    assert_type(value.value, output.OpenAdjacent | None)
    if value.value is not None:
        assert_type(
            value.value.root,
            output.OpenAdjacentValue
            | output.OpenAdjacentUnit
            | output.OpenAdjacentUnknown,
        )


def check_nested_flatten(value: output.NestedFlattenEnvelope) -> None:
    assert_type(value.value, output.OpenInternal)


def check_closed_external_flatten(
    value: output.OptionalFlattenClosedExternalEnvelope,
) -> None:
    assert_type(value.value, ClosedExternal | None)


def check_sync_responses(client: Client) -> None:
    internal = client.enums.internal()
    assert_type(internal, ApiResponse[output.OpenInternal, Any, ResponseHeaders])
    assert_type(internal.data, output.OpenInternal)

    optional = client.enums_optional.flatten_enum()
    assert_type(
        optional,
        ApiResponse[output.OptionalFlattenEnumEnvelope, Any, ResponseHeaders],
    )
    assert_type(optional.data.value, output.OpenInternal | None)


async def check_async_response(client: AsyncClient) -> None:
    response = await client.enums.internal()
    assert_type(response, ApiResponse[output.OpenInternal, Any, ResponseHeaders])
    assert_type(
        response.data.root,
        output.OpenInternalDog | output.OpenInternalBird | output.OpenInternalUnknown,
    )
