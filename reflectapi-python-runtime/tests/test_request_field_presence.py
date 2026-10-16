"""Request bytes preserve supplied fields and leave omitted defaults to the server."""

from typing import Any

import httpx
import pytest
from pydantic import BaseModel, Field

from reflectapi_runtime import AsyncClientBase, ClientBase, ReflectapiPartialModel


class NullableRequest(BaseModel):
    required_nullable: str | None
    optional_nullable: str | None = None
    defaulted: int = 7


class AliasedRequest(BaseModel):
    required_nullable: str | None = Field(alias="requiredAlias")
    optional_nullable: str | None = Field(default=None, alias="optionalAlias")


class PartialRequest(ReflectapiPartialModel):
    required_nullable: str | None
    optional_nullable: str | None = None
    ordinary: AliasedRequest | None = None


class NestedRequest(BaseModel):
    ordinary: AliasedRequest
    items: list[AliasedRequest]
    union: AliasedRequest | str
    partial: PartialRequest


def request_cases() -> list[Any]:
    aliased = AliasedRequest(requiredAlias=None, optionalAlias=None)
    return [
        pytest.param(
            NullableRequest(required_nullable=None),
            b'{"required_nullable":null}',
            id="required-null-and-unset-optionals",
        ),
        pytest.param(
            NullableRequest(required_nullable=None, optional_nullable=None),
            b'{"required_nullable":null,"optional_nullable":null}',
            id="explicit-optional-null",
        ),
        pytest.param(
            NullableRequest(required_nullable="value", defaulted=7),
            b'{"required_nullable":"value","defaulted":7}',
            id="explicit-default-is-sent",
        ),
        pytest.param(
            AliasedRequest(requiredAlias=None),
            b'{"requiredAlias":null}',
            id="required-null-alias",
        ),
        pytest.param(
            aliased,
            b'{"requiredAlias":null,"optionalAlias":null}',
            id="explicit-null-alias",
        ),
        pytest.param(
            PartialRequest(required_nullable=None),
            b'{"required_nullable":null}',
            id="partial-unset-optionals",
        ),
        pytest.param(
            PartialRequest(
                required_nullable=None,
                optional_nullable=None,
                ordinary=AliasedRequest(requiredAlias=None),
            ),
            b'{"required_nullable":null,"optional_nullable":null,'
            b'"ordinary":{"requiredAlias":null}}',
            id="partial-with-ordinary-model",
        ),
        pytest.param(
            NestedRequest(
                ordinary=aliased,
                items=[AliasedRequest(requiredAlias=None), aliased],
                union=aliased,
                partial=PartialRequest(required_nullable=None, optional_nullable=None),
            ),
            b'{"ordinary":{"requiredAlias":null,"optionalAlias":null},'
            b'"items":[{"requiredAlias":null},'
            b'{"requiredAlias":null,"optionalAlias":null}],'
            b'"union":{"requiredAlias":null,"optionalAlias":null},'
            b'"partial":{"required_nullable":null,"optional_nullable":null}}',
            id="ordinary-nesting-list-union-and-partial",
        ),
        pytest.param(
            [AliasedRequest(requiredAlias=None), aliased],
            b'[{"requiredAlias":null},{"requiredAlias":null,"optionalAlias":null}]',
            id="list-root-with-models",
        ),
        pytest.param(
            {"ordinary": AliasedRequest(requiredAlias=None), "explicit": None},
            b'{"ordinary":{"requiredAlias":null},"explicit":null}',
            id="dictionary-root-with-model",
        ),
    ]


@pytest.mark.parametrize("payload, expected", request_cases())
@pytest.mark.parametrize("body_argument", ["json_model", "json_data"])
def test_sync_request_field_presence(payload, expected, body_argument):
    captured = []

    def respond(request):
        captured.append(request.content)
        return httpx.Response(200, json={})

    with httpx.Client(transport=httpx.MockTransport(respond)) as transport:
        client = ClientBase("https://example.test", client=transport)
        client._make_request("/presence", **{body_argument: payload})

    assert captured == [expected]


@pytest.mark.parametrize("payload, expected", request_cases())
@pytest.mark.parametrize("body_argument", ["json_model", "json_data"])
@pytest.mark.asyncio
async def test_async_request_field_presence(payload, expected, body_argument):
    captured = []

    def respond(request):
        captured.append(request.content)
        return httpx.Response(200, json={})

    async with httpx.AsyncClient(transport=httpx.MockTransport(respond)) as transport:
        client = AsyncClientBase("https://example.test", client=transport)
        await client._make_request("/presence", **{body_argument: payload})

    assert captured == [expected]
