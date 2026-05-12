"""Partial-model base class for reflectapi-generated Pydantic models.

reflectapi schemas distinguish three states for nullable fields declared
as ``reflectapi::Option<T>``: ``Some(value)``, explicit ``null``, and
"key absent on the wire" (``Undefined``). TypeScript clients express
this natively (``field?: T | null``); Rust clients use the
``reflectapi::Option<T>`` enum. Python clients used to ship a custom
``ReflectapiOption[T]`` wrapper class to surface the same distinction.

The wrapper duplicated information Pydantic already tracks via
``model_fields_set`` and forced users to learn a parallel
``.unwrap`` / ``.is_undefined`` API. This module replaces it with a
``BaseModel`` mixin that:

- leaves field types as plain ``T | None`` (so Pydantic validates them
  exactly as it would any other nullable field),
- relies on ``model_fields_set`` — Pydantic's built-in record of which
  fields were *actually provided* during deserialise / construction —
  as the source of truth for "was this on the wire?",
- emits a wire payload that omits keys not in ``model_fields_set``,
  preserving the absent-vs-null distinction round-trip.

Codegen marks every generated class with at least one
``reflectapi::Option<T>`` field as inheriting from
:class:`ReflectapiPartialModel` instead of :class:`pydantic.BaseModel`.
"""

from __future__ import annotations

from typing import Any

from pydantic import BaseModel, model_serializer


class ReflectapiPartialModel(BaseModel):
    """Base class for reflectapi models with partial (three-state) fields.

    Wire-format guarantee: a field that was never explicitly set on the
    instance is omitted from the serialised output entirely. A field set
    to ``None`` is emitted as ``null``. A field set to a value is emitted
    as that value. This mirrors what TypeScript clients produce for
    ``field?: T | null`` and what Rust clients produce for
    ``reflectapi::Option<T>``.

    Usage on the read path:

    .. code-block:: python

        item = Item.model_validate({"name": "x"})  # snapshot key absent
        "snapshot" in item.model_fields_set        # False — was absent

        item = Item.model_validate({"name": "x", "snapshot": None})
        "snapshot" in item.model_fields_set        # True — explicit null
        item.snapshot is None                      # True

    Usage on the write path:

    .. code-block:: python

        Item(name="x").model_dump_json()
        # '{"name":"x"}'   — snapshot omitted because it wasn't set

        Item(name="x", snapshot=None).model_dump_json()
        # '{"name":"x","snapshot":null}'
    """

    # NOTE on ``model_fields_set`` and post-construction assignment:
    # Pydantic populates ``model_fields_set`` during construction
    # (kwargs) and deserialise (``model_validate``). Subsequent
    # attribute writes do **not** add to that set unless the model's
    # ``model_config`` enables ``validate_assignment=True``. The
    # generated client code emits ``ConfigDict(extra="ignore",
    # populate_by_name=True, validate_assignment=True)`` on every
    # partial class for this reason, so users can also do
    # ``m.snapshot = None`` after construction and have it land on the
    # wire.

    @model_serializer(mode="wrap")
    def _serialize_partial(
        self, handler: Any, info: Any | None = None
    ) -> dict[str, Any]:
        """Drop fields the caller never explicitly set.

        Pydantic populates ``model_fields_set`` with the *field names*
        of every field that was either present in the input dict during
        ``model_validate`` *or* passed as a keyword to ``__init__``.
        Defaults populated by Pydantic itself are excluded — which is
        exactly the "was this on the wire?" signal we need.

        Two complications the implementation has to handle:

        1. When ``by_alias=True`` is in effect, the handler returns a
           dict keyed by the field's serialization alias, not by its
           Python attribute name. ``model_fields_set`` always holds the
           Python name, so we expand it into the set of names that
           could plausibly appear in ``data`` (Python name + alias).

        2. RootModel and friends may return non-dict data (e.g. a bare
           string for a unit-variant enum). In that case there's nothing
           to filter; just pass it through.
        """
        data = handler(self)
        if not isinstance(data, dict):
            return data
        emit_keys: set[str] = set()
        model_fields = type(self).model_fields
        for name in self.model_fields_set:
            emit_keys.add(name)
            field_info = model_fields.get(name)
            if field_info is None:
                continue
            alias = field_info.serialization_alias or field_info.alias
            if alias:
                emit_keys.add(alias)
        return {key: value for key, value in data.items() if key in emit_keys}
