"""Replay serde's conformance samples through the generated Python client and
validate them against the generated OpenAPI spec.

`cargo test -p reflectapi-demo --test conformance` writes the client, the
spec and `samples.json` to `target/conformance/`. For each sample:

- request: a value decoded from the sample is sent as the sample;
- response: the sample, served back, decodes and re-encodes unchanged;
- schema: the sample validates against the route's OpenAPI request and
  response schemas.

A known divergence (listed in `tests/conformance.rs`) must fail; one that
passes fails the test until its entry is removed.
"""

import json
import pathlib
import sys
import typing
from collections.abc import Callable

import jsonschema
import pydantic
import pytest
from reflectapi_runtime.transport import Request, Response

ROOT = pathlib.Path(__file__).resolve().parents[2] / "target" / "conformance"
sys.path.insert(0, str(ROOT / "python"))

from conformance_client import Client

FIXTURE = json.loads((ROOT / "samples.json").read_text())
SPEC = json.loads((ROOT / "openapi.json").read_text())

SAMPLES = [
    pytest.param(case, i, sample, id=f"{case}-{i}")
    for case, entry in FIXTURE["cases"].items()
    for i, sample in enumerate(entry["samples"])
]


def divergence(case: str, sample: int, target: str, check: str) -> str | None:
    for d in FIXTURE["divergences"]:
        if (
            d["case"] == case
            and d["target"] == target
            and d["check"] == check
            and d["sample"] in (None, sample)
        ):
            return d["reason"]
    return None


def conform(case: str, sample: int, target: str, check: str, run: Callable) -> None:
    """Runs `run`, which raises on a mismatch, expecting it to fail exactly
    when the sample is a known divergence."""
    reason = divergence(case, sample, target, check)
    if reason is None:
        run()
        return
    try:
        run()
    # pydantic's ValidationError is a ValueError.
    except (AssertionError, ValueError, TypeError, jsonschema.ValidationError):
        pytest.xfail(reason)
    pytest.fail(
        f"listed as a divergence ({reason}) but passes; remove it from DIVERGENCES"
    )


class ServeSample:
    """Records the request body and responds with the sample."""

    def __init__(self, sample: object) -> None:
        self.body = json.dumps(sample).encode()
        self.path: str | None = None
        self.sent: object = None

    def request(self, request: Request) -> Response:
        self.path = request.path
        self.sent = json.loads(request.body)
        return Response(
            status=200, headers={"content-type": "application/json"}, body=self.body
        )


def route(client: Client, case: str):
    method = getattr(client.conformance, case)
    hints = typing.get_type_hints(method)
    # `data` is `Optional[T]`, which flattens a union `T` into its members.
    members = tuple(t for t in typing.get_args(hints["data"]) if t is not type(None))
    input_type = typing.Union[members]  # noqa: UP007
    (output_type, *_) = typing.get_args(hints["return"])
    return method, input_type, output_type


@pytest.mark.parametrize(("case", "i", "sample"), SAMPLES)
def test_request_is_the_sample(case: str, i: int, sample: object) -> None:
    def run() -> None:
        transport = ServeSample(sample)
        client = Client("http://test", client=transport)
        method, input_type, _ = route(client, case)
        method(pydantic.TypeAdapter(input_type).validate_json(json.dumps(sample)))
        assert transport.path == FIXTURE["cases"][case]["path"]
        assert transport.sent == sample

    conform(case, i, "python", "request", run)


@pytest.mark.parametrize(("case", "i", "sample"), SAMPLES)
def test_response_reencodes_as_the_sample(case: str, i: int, sample: object) -> None:
    def run() -> None:
        transport = ServeSample(sample)
        client = Client("http://test", client=transport)
        method, input_type, _ = route(client, case)
        # The request is only the means of getting the response here.
        response = method(
            pydantic.TypeAdapter(input_type).validate_json(json.dumps(sample))
        )
        body, _ = client._serialize_request_body(response.data)
        assert json.loads(body) == sample

    conform(case, i, "python", "response", run)


def openapi_schema(case: str, direction: str) -> dict:
    operation = SPEC["paths"][FIXTURE["cases"][case]["path"]]["post"]
    if direction == "request":
        content = operation["requestBody"]["content"]
    else:
        content = operation["responses"]["200"]["content"]
    # Refs point into `#/components`, so validate against a document that
    # carries them.
    return {**content["application/json"]["schema"], "components": SPEC["components"]}


@pytest.mark.parametrize("direction", ["request", "response"])
@pytest.mark.parametrize(("case", "i", "sample"), SAMPLES)
def test_openapi_schema_admits_the_sample(
    case: str, i: int, sample: object, direction: str
) -> None:
    def run() -> None:
        jsonschema.validate(
            sample,
            openapi_schema(case, direction),
            cls=jsonschema.Draft202012Validator,
        )

    conform(case, i, "openapi", f"schema-{direction}", run)
