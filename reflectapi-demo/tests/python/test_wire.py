"""Replay serde-produced JSON through the generated codegen-coverage client.

`cargo test -p reflectapi-demo --test codegen_coverage` writes the client and
`wire_samples.json`. Each sample must reach the transport unchanged, and the
same bytes served back by the transport must parse and re-serialize unchanged,
and the model's JSON schemas must describe it.
"""

import asyncio
import json
import pathlib
import sys

import jsonschema
import pytest
from pydantic.json_schema import JsonSchemaMode
from reflectapi_runtime.transport import Request, Response

CLIENT_DIR = (
    pathlib.Path(__file__).resolve().parents[2] / "target" / "codegen-coverage-client"
)
sys.path.insert(0, str(CLIENT_DIR))

from codegen_coverage_client import AsyncClient, Client  # noqa: E402
from codegen_coverage_client.codegen_coverage.wire import WireRequest  # noqa: E402

SAMPLES = json.loads((CLIENT_DIR / "wire_samples.json").read_text())
SAMPLE_IDS = [json.dumps(sample["value"]) for sample in SAMPLES]


class ServeSample:
    """Records the request body and responds with the serde sample."""

    def __init__(self, sample: object) -> None:
        self.body = json.dumps(sample).encode()
        self.sent: object = None

    def _respond(self, request: Request) -> Response:
        self.sent = json.loads(request.body)
        return Response(
            status=200, headers={"content-type": "application/json"}, body=self.body
        )

    def request(self, request: Request) -> Response:
        return self._respond(request)


class AsyncServeSample(ServeSample):
    async def request(self, request: Request) -> Response:
        return self._respond(request)


def reserialize(model: WireRequest) -> object:
    return json.loads(model.model_dump_json(by_alias=True))


@pytest.mark.parametrize("sample", SAMPLES, ids=SAMPLE_IDS)
def test_sync_client_preserves_serde_wire_shape(sample: dict) -> None:
    transport = ServeSample(sample)
    client = Client("http://test", client=transport)

    response = client.coverage.wire(WireRequest.model_validate(sample))

    assert transport.sent == sample
    assert reserialize(response.data) == sample


@pytest.mark.parametrize("sample", SAMPLES, ids=SAMPLE_IDS)
def test_async_client_preserves_serde_wire_shape(sample: dict) -> None:
    transport = AsyncServeSample(sample)
    client = AsyncClient("http://test", client=transport)

    response = asyncio.run(client.coverage.wire(WireRequest.model_validate(sample)))

    assert transport.sent == sample
    assert reserialize(response.data) == sample


@pytest.mark.parametrize("mode", ["validation", "serialization"])
@pytest.mark.parametrize("sample", SAMPLES, ids=SAMPLE_IDS)
def test_json_schema_describes_serde_wire_shape(
    sample: dict, mode: JsonSchemaMode
) -> None:
    schema = WireRequest.model_json_schema(mode=mode)
    jsonschema.validate(sample, schema, cls=jsonschema.Draft202012Validator)
