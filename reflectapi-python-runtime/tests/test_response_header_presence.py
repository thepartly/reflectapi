"""Header absence does not weaken the same model's required JSON body fields."""

from datetime import datetime

import httpx
import pytest
from pydantic import BaseModel, ConfigDict, Field
from pydantic import ValidationError as PydanticValidationError

from reflectapi_runtime import ApplicationError, AsyncClientBase, ClientBase


class RequiredNullableHeaders(BaseModel):
    # Exercise errors reported by Python field name as well as aliased input.
    model_config = ConfigDict(loc_by_alias=False)

    expires_at: datetime | None = Field(validation_alias="x-expires-at")
    request_id: str | None = Field(alias="x-request-id")
    retry_after: str | None = Field(validation_alias="retry-after")


def assert_headers(headers):
    assert isinstance(headers, RequiredNullableHeaders)
    assert headers.expires_at is None
    assert headers.request_id == "req-1"
    assert headers.retry_after is None


@pytest.mark.parametrize("status", [200, 503])
@pytest.mark.parametrize("malformed", [False, True])
def test_sync_required_nullable_response_headers(status, malformed):
    wire_headers = {"X-Request-ID": "req-1"}
    if malformed:
        wire_headers["X-Expires-At"] = "invalid-datetime"

    def respond(request):
        return httpx.Response(status, headers=wire_headers, json={})

    # Body validation still requires nullable keys when headers share the model.
    with pytest.raises(PydanticValidationError):
        RequiredNullableHeaders.model_validate({"x-request-id": "req-1"})

    with httpx.Client(transport=httpx.MockTransport(respond)) as transport:
        client = ClientBase("https://example.test", client=transport)
        if status == 503:
            with pytest.raises(ApplicationError) as exc:
                client._make_request(
                    "/headers", response_headers_model=RequiredNullableHeaders
                )
            result = exc.value
            assert result.status_code == 503
        else:
            result = client._make_request(
                "/headers", response_headers_model=RequiredNullableHeaders
            )
        assert_headers(result.headers)
        if malformed:
            assert result.metadata.headers["x-expires-at"] == "invalid-datetime"


@pytest.mark.parametrize("status", [200, 503])
@pytest.mark.parametrize("malformed", [False, True])
@pytest.mark.asyncio
async def test_async_required_nullable_response_headers(status, malformed):
    wire_headers = {"X-Request-ID": "req-1"}
    if malformed:
        wire_headers["X-Expires-At"] = "invalid-datetime"

    def respond(request):
        return httpx.Response(status, headers=wire_headers, json={})

    with pytest.raises(PydanticValidationError):
        RequiredNullableHeaders.model_validate({"x-request-id": "req-1"})

    async with httpx.AsyncClient(transport=httpx.MockTransport(respond)) as transport:
        client = AsyncClientBase("https://example.test", client=transport)
        if status == 503:
            with pytest.raises(ApplicationError) as exc:
                await client._make_request(
                    "/headers", response_headers_model=RequiredNullableHeaders
                )
            result = exc.value
            assert result.status_code == 503
        else:
            result = await client._make_request(
                "/headers", response_headers_model=RequiredNullableHeaders
            )
        assert_headers(result.headers)
        if malformed:
            assert result.metadata.headers["x-expires-at"] == "invalid-datetime"
