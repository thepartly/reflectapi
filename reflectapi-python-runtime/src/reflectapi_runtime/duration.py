"""Wire-format adapter for Rust's ``std::time::Duration``.

serde writes a ``Duration`` as ``{"secs": <u64>, "nanos": <u32>}``, which
pydantic's ``timedelta`` doesn't read, and with nanoseconds, which a
``timedelta`` can't hold.

``ReflectapiDuration``, the annotation generated clients use for
``Duration``, is a ``timedelta``, so any ``timedelta`` can be assigned to
such a field:

- **Reading** (server → client): ``{"secs", "nanos"}`` becomes a
  ``WireDuration``: a ``timedelta``, truncated to the microsecond, that keeps
  the ``secs`` and ``nanos`` it was read from. A ``timedelta``, or anything
  else pydantic reads as one, is read as pydantic reads it.
- **Writing** (client → server): a ``WireDuration`` is written as the exact
  ``secs`` and ``nanos`` it was read from. Any other ``timedelta``, including
  the result of arithmetic, is written as its whole microseconds.

Two values that differ only below a microsecond compare equal, as their
``timedelta`` parts are equal; ``WireDuration.wire`` tells them apart.
A ``Duration`` longer than ``timedelta.max`` (about 2.7 million years), such
as ``Duration::MAX``, reads as ``timedelta.max`` and is still written back
exactly. ``Duration`` is unsigned, so a server rejects a negative ``timedelta``.
"""

from __future__ import annotations

from datetime import timedelta
from typing import TYPE_CHECKING, Annotated, Any

from pydantic_core import core_schema

if TYPE_CHECKING:
    from pydantic import GetCoreSchemaHandler


class WireDuration(timedelta):
    """A ``timedelta`` read from the wire, written back exactly as it was read."""

    __slots__ = ("_wire",)
    _wire: tuple[int, int] | None

    @classmethod
    def _read(cls, secs: int, nanos: int) -> WireDuration:
        try:
            out = cls(seconds=secs, microseconds=nanos // 1_000)
        except OverflowError:
            limit = timedelta.max if secs > 0 else timedelta.min
            out = cls(limit.days, limit.seconds, limit.microseconds)
        out._wire = (secs, nanos)
        return out

    @property
    def wire(self) -> tuple[int, int] | None:
        """The ``(secs, nanos)`` this was read from, or ``None`` for a new value."""
        return getattr(self, "_wire", None)

    def __reduce_ex__(self, protocol: Any) -> Any:
        # copy, deepcopy, pickle and `model_copy(deep=True)` keep the wire value.
        return (_restore, (super().__reduce_ex__(protocol), self.wire))


def _restore(reduced: Any, wire: tuple[int, int] | None) -> WireDuration:
    constructor, args = reduced[0], reduced[1]
    out: WireDuration = constructor(*args)
    if wire is not None:
        out._wire = wire
    return out


def _is_int(value: Any) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def _to_wire(value: timedelta) -> dict[str, int]:
    wire = getattr(value, "_wire", None)
    if wire is not None:
        secs, nanos = wire
        return {"secs": secs, "nanos": nanos}
    # Integer arithmetic: a float would lose digits on long durations.
    total_us = (value.days * 86_400 + value.seconds) * 1_000_000 + value.microseconds
    secs, micros = divmod(total_us, 1_000_000)
    return {"secs": secs, "nanos": micros * 1_000}


class _KeepWireValue:
    """Pydantic schema: read serde's shape into a ``WireDuration``, write it back."""

    def __get_pydantic_core_schema__(
        self, source: Any, handler: GetCoreSchemaHandler
    ) -> core_schema.CoreSchema:
        def validate(
            value: Any, parse: core_schema.ValidatorFunctionWrapHandler
        ) -> timedelta:
            if isinstance(value, WireDuration):
                return value
            if isinstance(value, dict):
                secs, nanos = value.get("secs", 0), value.get("nanos", 0)
                if _is_int(secs) and _is_int(nanos):
                    return WireDuration._read(secs, nanos)
                if isinstance(secs, (int, float)) and isinstance(nanos, (int, float)):
                    return timedelta(seconds=secs, microseconds=nanos / 1_000)
            return parse(value)

        def serialize(
            value: timedelta,
            write: core_schema.SerializerFunctionWrapHandler,
            info: core_schema.SerializationInfo,
        ) -> Any:
            if info.mode_is_json():
                return _to_wire(value)
            return write(value)

        return core_schema.no_info_wrap_validator_function(
            validate,
            core_schema.timedelta_schema(),
            serialization=core_schema.wrap_serializer_function_ser_schema(
                serialize,
                info_arg=True,
                schema=core_schema.timedelta_schema(),
                return_schema=core_schema.typed_dict_schema(
                    {
                        "secs": core_schema.typed_dict_field(core_schema.int_schema()),
                        "nanos": core_schema.typed_dict_field(core_schema.int_schema()),
                    }
                ),
            ),
        )


ReflectapiDuration = Annotated[timedelta, _KeepWireValue()]
"""A ``timedelta`` in serde's ``{secs, nanos}`` shape, written back exactly as read."""
