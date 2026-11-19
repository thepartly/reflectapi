"""A ``datetime`` that remembers the text it was read from.

chrono keeps nanoseconds, and serde writes them:
``2026-11-19T10:20:30.123456789Z``. Python's ``datetime`` holds
microseconds, so decoding into a plain ``datetime`` and encoding it again
sends a different timestamp to the server, which breaks it as a version
token, a cursor or a key.

``ReflectapiDateTime`` is the type generated clients use for
``chrono::DateTime`` and ``chrono::NaiveDateTime``:

- **Reading** (server → client): the text is parsed as pydantic parses a
  ``datetime``, and kept on the value. It is a ``datetime``, with the
  microseconds, so ``isinstance(x, datetime)``, comparisons and arithmetic
  work as before.
- **Writing** (client → server): a value read from the wire is written as
  the exact text it was read from. Any other ``datetime``, including the
  result of arithmetic or ``replace()``, is written as pydantic writes one.

Two values that differ only below a microsecond compare equal, as their
``datetime`` parts are equal; ``.wire`` tells them apart.
"""

from __future__ import annotations

from datetime import datetime
from typing import TYPE_CHECKING, Any

from pydantic_core import core_schema

if TYPE_CHECKING:
    from pydantic import GetCoreSchemaHandler


class ReflectapiDateTime(datetime):
    """A ``datetime`` read from the wire, written back as the text it was read from."""

    __slots__ = ("_wire",)
    _wire: str | None

    @classmethod
    def _read(cls, value: datetime, wire: str) -> ReflectapiDateTime:
        out = cls(
            value.year,
            value.month,
            value.day,
            value.hour,
            value.minute,
            value.second,
            value.microsecond,
            value.tzinfo,
            fold=value.fold,
        )
        out._wire = wire
        return out

    @property
    def wire(self) -> str | None:
        """The text this value was read from, or ``None`` for a new value."""
        return getattr(self, "_wire", None)

    def __reduce_ex__(self, protocol: Any) -> Any:
        # copy, deepcopy, pickle and `model_copy(deep=True)` keep the text.
        return (_restore, (super().__reduce_ex__(protocol), self.wire))

    @classmethod
    def __get_pydantic_core_schema__(
        cls, source: Any, handler: GetCoreSchemaHandler
    ) -> core_schema.CoreSchema:
        def validate(
            value: Any, parse: core_schema.ValidatorFunctionWrapHandler
        ) -> datetime:
            if isinstance(value, ReflectapiDateTime):
                return value
            parsed: datetime = parse(value)
            if isinstance(value, str):
                return cls._read(parsed, value)
            return parsed

        def serialize(
            value: datetime,
            write: core_schema.SerializerFunctionWrapHandler,
            info: core_schema.SerializationInfo,
        ) -> Any:
            wire = getattr(value, "_wire", None)
            if wire is not None and info.mode_is_json():
                return wire
            return write(value)

        return core_schema.no_info_wrap_validator_function(
            validate,
            core_schema.datetime_schema(),
            serialization=core_schema.wrap_serializer_function_ser_schema(
                serialize, info_arg=True, schema=core_schema.datetime_schema()
            ),
        )


def _restore(reduced: Any, wire: str | None) -> ReflectapiDateTime:
    constructor, args = reduced[0], reduced[1]
    out: ReflectapiDateTime = constructor(*args)
    if wire is not None:
        out._wire = wire
    return out
