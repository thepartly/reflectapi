"""Wire-format adapter for Rust's ``std::time::Duration``.

serde serialises ``Duration`` as ``{"secs": <u64>, "nanos": <u32>}``.
Pydantic v2's built-in ``timedelta`` validator accepts ISO-8601 strings,
ints, and floats — it does **not** accept the ``{secs, nanos}`` dict
that the server actually sends. The bare ``timedelta`` annotation
emitted by older codegen therefore failed validation on every response
that carried a ``Duration`` field.

``ReflectapiDuration`` is the type the generated client uses instead. It
preserves the ergonomic Python ``timedelta`` API (``td.total_seconds()``,
arithmetic, comparisons) and round-trips the serde shape:

- **Validating** (server → client): a ``{"secs": …, "nanos": …}`` dict
  is converted to a ``timedelta``; ints/floats and existing
  ``timedelta`` instances pass through unchanged so users can construct
  models from Python values directly.
- **Serialising** (client → server): the ``timedelta`` is written back
  as ``{"secs": <int>, "nanos": <int>}`` so the wire payload round-trips
  cleanly through ``serde::Deserialize`` on the server.

Precision caveat: ``timedelta`` stores microseconds, so the bottom three
decimal digits of ``nanos`` are truncated on round-trip. If a server
ever sends a ``nanos`` value not divisible by 1 000 (sub-microsecond),
the recovered ``nanos`` will be the closest microsecond-aligned value.
For the durations reflectapi APIs typically carry (timeouts, retry
hints, rate-limit windows) this is well below the noise floor.
"""

from __future__ import annotations

from datetime import timedelta
from typing import Annotated, Any

from pydantic import BeforeValidator, PlainSerializer

_NS_PER_SECOND = 1_000_000_000


def _validate(value: Any) -> Any:
    """Accept serde's ``{secs, nanos}`` dict; pass other forms through."""
    if isinstance(value, dict):
        secs = value.get("secs", 0)
        nanos = value.get("nanos", 0)
        if not isinstance(secs, (int, float)) or not isinstance(nanos, (int, float)):
            return value  # let Pydantic raise its own validation error
        return timedelta(seconds=secs, microseconds=nanos / 1_000)
    return value


def _serialise(value: timedelta) -> dict[str, int]:
    """Emit the wire shape Rust's ``serde::Deserialize<Duration>`` expects."""
    if not isinstance(value, timedelta):
        # Pydantic already coerced; if anything else slips through, surface
        # the failure rather than papering over it.
        raise TypeError(
            f"ReflectapiDuration serialiser expected a timedelta, got {type(value).__name__}"
        )
    # Pull total nanoseconds out of the timedelta in two integer pieces
    # so we never round-trip through float and lose precision below the
    # microsecond level.
    total_us = (value.days * 86_400 + value.seconds) * 1_000_000 + value.microseconds
    secs, micros = divmod(total_us, 1_000_000)
    nanos = micros * 1_000
    return {"secs": secs, "nanos": nanos}


ReflectapiDuration = Annotated[
    timedelta,
    BeforeValidator(_validate),
    PlainSerializer(_serialise, when_used="json"),
]
"""Pydantic field type for ``std::time::Duration`` — see module docstring."""
