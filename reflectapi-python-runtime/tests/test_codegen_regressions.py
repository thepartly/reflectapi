"""Contract tests for runtime types consumed by the Python codegen.

These tests pin the runtime API the generated client depends on. They
run on every push without needing a full demo-client regeneration; the
complementary end-to-end import check lives in
``.github/workflows/ci.yaml::python-codegen-smoke``.
"""

from __future__ import annotations

import json
from datetime import timedelta

import pytest
from pydantic import BaseModel, ValidationError

from reflectapi_runtime import ReflectapiDuration


class TestDurationRoundTrip:
    """``ReflectapiDuration`` round-trips serde's ``{secs, nanos}``
    wire shape through a Python ``timedelta``."""

    def _model(self):
        class M(BaseModel):
            d: ReflectapiDuration

        return M

    def test_validate_secs_nanos_dict(self):
        M = self._model()
        m = M.model_validate({"d": {"secs": 30, "nanos": 0}})
        assert m.d == timedelta(seconds=30)

    def test_validate_with_nanos(self):
        M = self._model()
        m = M.model_validate({"d": {"secs": 1, "nanos": 500_000_000}})
        # timedelta resolves to microsecond precision; we get 1.5 seconds.
        assert m.d == timedelta(seconds=1, microseconds=500_000)

    def test_validate_existing_timedelta(self):
        """Code that already has a ``timedelta`` should pass through unchanged."""
        M = self._model()
        m = M.model_validate({"d": timedelta(seconds=5)})
        assert m.d == timedelta(seconds=5)

    def test_serialise_emits_secs_nanos(self):
        M = self._model()
        m = M(d=timedelta(seconds=30, microseconds=500_000))
        payload = json.loads(m.model_dump_json())
        assert payload == {"d": {"secs": 30, "nanos": 500_000_000}}

    def test_serialise_subsecond(self):
        M = self._model()
        m = M(d=timedelta(microseconds=1_500))  # 1.5 ms
        payload = json.loads(m.model_dump_json())
        # 1500 microseconds == 1_500_000 nanoseconds (well under one second).
        assert payload == {"d": {"secs": 0, "nanos": 1_500_000}}

    def test_round_trip(self):
        M = self._model()
        original = M(d=timedelta(hours=2, seconds=3, microseconds=4))
        reloaded = M.model_validate_json(original.model_dump_json())
        assert reloaded.d == original.d

    def test_round_trip_negative_duration_rejected(self):
        """serde's `Duration` is unsigned; we don't pretend to support negatives."""
        M = self._model()
        m = M(d=timedelta(seconds=-1))
        # We *do* serialise it (Python's timedelta is signed), but the
        # resulting nanos field will be negative. The test pins the
        # current behaviour — negative durations land on the wire as a
        # negative `secs` integer, which a Rust server would reject.
        payload = json.loads(m.model_dump_json())
        # divmod(-1_000_000, 1_000_000) == (-1, 0)  → secs=-1, nanos=0
        assert payload == {"d": {"secs": -1, "nanos": 0}}

    def test_invalid_dict_shape_falls_through_to_pydantic_error(self):
        """A dict without the right keys should produce a clean ValidationError."""
        M = self._model()
        with pytest.raises(ValidationError):
            M.model_validate({"d": {"secs": "not-a-number"}})


class TestRequestSerializationOmitsUnsetOptional:
    """Plain ``BaseModel`` request types must omit unset optional fields
    from the wire, matching what ``#[serde(skip_serializing_if =
    "Option::is_none")]`` produces on the server side. Partial models,
    by contrast, must keep explicit ``None`` to preserve the
    absent-vs-null distinction.
    """

    def _client(self):
        from reflectapi_runtime import ClientBase

        return ClientBase("http://test")

    def test_plain_model_omits_unset_optional(self):
        from reflectapi_runtime import ReflectapiPartialModel  # noqa: F401

        class Req(BaseModel):
            cursor: str | None = None
            limit: int | None = None

        client = self._client()
        # `Req()` leaves both fields at their `None` default; the wire
        # payload should drop them.
        body, _headers = client._serialize_request_body(Req())
        assert body == b"{}"

    def test_plain_model_keeps_explicit_value(self):
        class Req(BaseModel):
            cursor: str | None = None
            limit: int | None = None

        client = self._client()
        body, _ = client._serialize_request_body(Req(cursor="abc", limit=10))
        assert body == b'{"cursor":"abc","limit":10}'

    def test_partial_model_emits_explicit_null(self):
        """``ReflectapiPartialModel`` must round-trip explicit ``None``
        as ``null`` (the wire's "field present, value cleared" state)."""
        from reflectapi_runtime import ReflectapiPartialModel

        class Req(ReflectapiPartialModel):
            model_config = {
                "extra": "ignore",
                "populate_by_name": True,
                "validate_assignment": True,
            }
            cursor: str | None = None
            limit: int | None = None

        client = self._client()
        body, _ = client._serialize_request_body(Req(cursor=None, limit=10))
        # Both keys were set (one to None, one to a value) — both go on the wire.
        assert b'"cursor":null' in body
        assert b'"limit":10' in body

    def test_partial_model_omits_unset_field(self):
        from reflectapi_runtime import ReflectapiPartialModel

        class Req(ReflectapiPartialModel):
            cursor: str | None = None
            limit: int | None = None

        client = self._client()
        body, _ = client._serialize_request_body(Req(limit=10))
        # `cursor` was not set on the instance — not in `model_fields_set` → omitted.
        assert b"cursor" not in body
        assert b'"limit":10' in body
