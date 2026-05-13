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
