"""Serialization helpers used by generated ReflectAPI clients."""

from __future__ import annotations

from collections.abc import Callable
from typing import Any

from pydantic import BaseModel, model_serializer, model_validator

VariantHandler = Callable[[Any], Any] | str
VariantSerializers = dict[str, tuple[Callable[[Any], bool], Callable[[Any], Any]]]


class UnknownEnumValue(BaseModel):
    """An output enum case whose wire tag was not in the generated schema.

    Generated clients construct this after checking the wire discriminator. A
    raw object cannot be validated directly as this case, which keeps malformed
    known cases from silently falling through a Pydantic union.
    """

    tag: str
    raw: Any

    @model_validator(mode="before")
    @classmethod
    def _instance_only(cls, value: Any) -> Any:
        if not isinstance(value, cls):
            raise ValueError("Unknown enum cases are created from unfamiliar wire tags")
        return value

    @model_serializer(mode="plain")
    def _serialize_raw(self) -> Any:
        return self.raw


def classify_tagged_enum(
    data: Any,
    known_tags: frozenset[str],
    *,
    representation: str,
    tag_field: str = "",
    content_field: str = "",
    fallback_tag: str | None = None,
    unknown_type: type[UnknownEnumValue] | None = None,
    unit_tags: frozenset[str] = frozenset(),
) -> Any:
    """Classify an unfamiliar valid tag without masking other validation errors."""
    if isinstance(data, UnknownEnumValue):
        return data
    if representation == "external":
        if isinstance(data, str):
            tag = data
        elif isinstance(data, dict) and len(data) == 1:
            tag = next(iter(data))
        else:
            return data
    elif isinstance(data, dict):
        if tag_field not in data or not isinstance(data[tag_field], str):
            raise ValueError("Tagged enum requires a string discriminator")
        tag = data[tag_field]
    else:
        return data
    if isinstance(tag, str) and tag in unit_tags and isinstance(data, dict):
        if representation == "adjacent" and data.get(content_field) is not None:
            raise ValueError("Unit variant cannot carry non-null content")
        if representation == "external":
            if data[tag] is not None:
                raise ValueError("Unit variant cannot carry non-null content")
            return tag
    if not isinstance(tag, str) or tag in known_tags:
        return data
    if (
        representation == "adjacent"
        and unknown_type is None
        and fallback_tag is not None
        and data.get(content_field) is not None
    ):
        raise ValueError("Fallback unit variant cannot carry non-null content")
    if unknown_type is not None:
        return unknown_type.model_construct(tag=tag, raw=data)
    if fallback_tag is not None:
        if representation == "external":
            return fallback_tag
        return {tag_field: fallback_tag}
    return data


def parse_externally_tagged(
    data: Any,
    variants: dict[str, VariantHandler],
    types: tuple[type[Any], ...],
    enum_name: str,
    *,
    fallback_tag: str | None = None,
    unknown_type: type[UnknownEnumValue] | None = None,
    strict_unit: bool = False,
) -> Any:
    """Parse an externally tagged enum from ``{"Variant": value}`` format."""
    if types and isinstance(data, types):
        return data
    data = classify_tagged_enum(
        data,
        frozenset(variants),
        representation="external",
        fallback_tag=fallback_tag,
        unknown_type=unknown_type,
    )
    if isinstance(data, UnknownEnumValue):
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
                if strict_unit and value is not None:
                    raise ValueError(f"Unit variant {key} cannot carry a payload")
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
    if isinstance(root, UnknownEnumValue):
        return root.raw
    for _variant_name, (check, serialize) in serializers.items():
        if check(root):
            return serialize(root)
    raise ValueError(f"Cannot serialize {enum_name} variant: {type(root)}")
