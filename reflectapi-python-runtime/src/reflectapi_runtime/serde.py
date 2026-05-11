"""Serialization helpers used by generated ReflectAPI clients."""

from __future__ import annotations

from collections.abc import Callable
from typing import Any


VariantHandler = Callable[[Any], Any] | str
VariantSerializers = dict[str, tuple[Callable[[Any], bool], Callable[[Any], Any]]]


def parse_externally_tagged(
    data: Any,
    variants: dict[str, VariantHandler],
    types: tuple[type[Any], ...],
    enum_name: str,
) -> Any:
    """Parse an externally tagged enum from ``{"Variant": value}`` format."""
    if types and isinstance(data, types):
        return data
    if isinstance(data, str) and data in variants:
        handler = variants[data]
        if handler == "_unit":
            return data
    if isinstance(data, dict):
        if len(data) != 1:
            raise ValueError("Externally tagged enum must have exactly one key")
        key, value = next(iter(data.items()))
        if key in variants:
            handler = variants[key]
            if handler == "_unit":
                return key
            if callable(handler):
                return handler(value)
    raise ValueError(f"Unknown variant for {enum_name}: {data}")


def serialize_externally_tagged(
    root: Any,
    serializers: VariantSerializers,
    enum_name: str,
) -> Any:
    """Serialize an externally tagged enum to ``{"Variant": value}`` format."""
    for _variant_name, (check, serialize) in serializers.items():
        if check(root):
            return serialize(root)
    raise ValueError(f"Cannot serialize {enum_name} variant: {type(root)}")
