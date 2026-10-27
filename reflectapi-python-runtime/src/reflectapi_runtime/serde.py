"""Serialization helpers used by generated ReflectAPI clients."""

from __future__ import annotations

import inspect
from collections.abc import Callable, Sequence
from typing import TYPE_CHECKING, Any

from pydantic import BaseModel, model_serializer, model_validator

if TYPE_CHECKING:
    from pydantic import GetJsonSchemaHandler
    from pydantic.json_schema import JsonSchemaValue
    from pydantic_core import CoreSchema

VariantHandler = Callable[[Any], Any] | str
VariantSerializers = dict[str, tuple[Callable[[Any], bool], Callable[[Any], Any]]]

# Marks the JSON schema of an open enum's unknown case, which the enum drops.
_UNKNOWN_VARIANT = "x-reflectapi-unknown-variant"


class UnknownEnumValue(BaseModel):
    """An output enum case whose wire tag was not in the generated schema.

    Generated clients construct this after checking the wire discriminator. A
    raw object cannot be validated directly as this case, which keeps malformed
    known cases from silently falling through a Pydantic union.

    It is left out of JSON schemas: an open enum is described by its known
    variants, as the server's own schema describes it. Its own schema is a
    marker that the enum's schema (``known_variants_json_schema``) drops.
    """

    tag: str
    raw: Any

    @classmethod
    def __get_pydantic_json_schema__(
        cls, core_schema: CoreSchema, handler: GetJsonSchemaHandler
    ) -> JsonSchemaValue:
        return {_UNKNOWN_VARIANT: True}

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


# Pydantic builds a JSON schema from a model's fields and never runs its
# custom validators or serializers. The generated classes below reshape the
# wire value in those hooks, so they also describe the wire shape through
# ``__get_pydantic_json_schema__``, which calls one of these helpers.

_WRAPPER_SCHEMAS = frozenset(
    ("function-before", "function-after", "function-wrap", "definitions")
)


def _unwrap(schema: Any) -> Any:
    while schema.get("type") in _WRAPPER_SCHEMAS and "schema" in schema:
        schema = schema["schema"]
    return schema


def _root_choices(core_schema: CoreSchema) -> list[CoreSchema]:
    """The members of a ``RootModel[Union[...]]`` root type, in declaration order."""
    schema = _unwrap(core_schema)
    if schema.get("type") == "model":
        schema = _unwrap(schema["schema"])
    if schema.get("type") != "union":
        return [schema]
    return [c[0] if isinstance(c, tuple) else c for c in schema["choices"]]


def _member_json_schema(
    handler: GetJsonSchemaHandler, member: CoreSchema
) -> JsonSchemaValue:
    """A union member's schema, as a ``$ref`` when the member is a model.

    The handler inlines a schema it is given directly; a reference to the
    definition that the enum's default schema already registered keeps it
    shared.
    """
    ref = member.get("ref")
    if ref is None:
        return handler(member)
    return handler({"type": "definition-ref", "schema_ref": ref})


def _resolve(
    handler: GetJsonSchemaHandler, schema: JsonSchemaValue
) -> JsonSchemaValue | None:
    """The definition a ``$ref`` points at, or ``None`` if not generated yet."""
    try:
        return handler.resolve_ref_schema(schema)
    except LookupError:
        return None


def known_variants_json_schema(
    core_schema: CoreSchema, handler: GetJsonSchemaHandler
) -> JsonSchemaValue:
    """Describe an open enum by its known variants, without the unknown case.

    The unknown case is the last member of the generated root union.
    """
    schema = dict(handler.resolve_ref_schema(handler(core_schema)))
    choices = _root_choices(core_schema)
    unknown = _member_json_schema(handler, choices[-1])
    for union in ("anyOf", "oneOf"):
        members = schema.get(union)
        if not members:
            continue
        known = [m for m in members if m != unknown and _UNKNOWN_VARIANT not in m]
        if len(known) == 1:
            del schema[union]
            schema = {**known[0], **schema}
        else:
            schema[union] = known
        break
    return schema


def _model_metadata(
    core_schema: CoreSchema, default: JsonSchemaValue
) -> JsonSchemaValue:
    """The enum class's own title and description.

    The default schema of a RootModel over a single model is that model's
    schema, so its title and description come from the class instead.
    """
    cls = _unwrap(core_schema).get("cls")
    if cls is None:
        return {k: default[k] for k in ("title", "description") if k in default}
    metadata: JsonSchemaValue = {"title": cls.model_config.get("title") or cls.__name__}
    doc = cls.__dict__.get("__doc__")
    if doc:
        metadata["description"] = inspect.cleandoc(doc)
    return metadata


def _tuple_items(
    handler: GetJsonSchemaHandler, variant: CoreSchema
) -> list[Any] | None:
    """The schemas of a tuple variant class's ``field_0``, ``field_1``, ..."""
    resolved = _resolve(handler, _member_json_schema(handler, variant))
    if resolved is None:
        return None
    properties = resolved.get("properties", {})
    items = []
    for index in range(len(properties)):
        item = dict(properties[f"field_{index}"])
        item.pop("title", None)
        items.append(item)
    return items


def externally_tagged_json_schema(
    core_schema: CoreSchema,
    handler: GetJsonSchemaHandler,
    variants: Sequence[tuple[str, str]],
) -> JsonSchemaValue:
    """Describe an externally tagged enum as it appears on the wire.

    ``variants`` lists ``(wire_name, shape)`` in the order of the RootModel's
    union members, where ``shape`` is ``"unit"`` (``"Name"``), ``"newtype"``
    (``{"Name": value}``), ``"tuple"`` (``{"Name": [a, b]}``) or ``"struct"``
    (``{"Name": {...}}``). Members after the listed variants (the unknown case
    of an open enum) are left out.
    """
    default = handler(core_schema)
    choices = _root_choices(core_schema)
    if len(choices) < len(variants):
        return default
    resolved = _resolve(handler, default) or {}
    alternatives: list[JsonSchemaValue] = []
    for (tag, shape), choice in zip(variants, choices, strict=False):
        if shape == "unit":
            alternatives.append({"const": tag, "type": "string"})
            continue
        if shape == "struct":
            payload = _member_json_schema(handler, choice)
        else:
            items = _tuple_items(handler, choice)
            if items is None:
                return default
            if shape == "newtype":
                payload = items[0]
            else:
                payload = {
                    "type": "array",
                    "prefixItems": items,
                    "minItems": len(items),
                    "maxItems": len(items),
                }
        alternatives.append(
            {"type": "object", "properties": {tag: payload}, "required": [tag]}
        )
    schema: JsonSchemaValue = (
        alternatives[0] if len(alternatives) == 1 else {"oneOf": alternatives}
    )
    schema.update(_model_metadata(core_schema, resolved))
    return schema


def _flattened_alternative(alternative: JsonSchemaValue) -> JsonSchemaValue:
    # A flattened unit variant of an externally tagged enum is a key with a
    # null value, not a bare string.
    if alternative.get("type") == "string" and "const" in alternative:
        tag = alternative["const"]
        return {
            "type": "object",
            "properties": {tag: {"type": "null"}},
            "required": [tag],
        }
    return alternative


def flattened_enum_json_schema(
    core_schema: CoreSchema,
    handler: GetJsonSchemaHandler,
    field: str,
    keys: Sequence[str],
    *,
    optional: bool = False,
) -> JsonSchemaValue:
    """Describe a struct whose enum field ``field`` is flattened into it.

    The enum's keys sit next to the struct's own, so the schema is the
    struct's other fields combined with the enum's schema. ``keys`` are the
    keys that mark the enum as present (its tag, or the variant names of an
    externally tagged enum); an ``optional`` group may also be absent.
    """
    parent = dict(handler.resolve_ref_schema(handler(core_schema)))
    properties = dict(parent.get("properties", {}))
    enum_schema = properties.pop(field, None)
    if enum_schema is None:
        return parent
    parent["properties"] = properties
    required = [name for name in parent.get("required", []) if name != field]
    if required:
        parent["required"] = required
    else:
        parent.pop("required", None)
    if optional and "anyOf" in enum_schema:
        present = [s for s in enum_schema["anyOf"] if s.get("type") != "null"]
        enum_schema = present[0] if len(present) == 1 else {"anyOf": present}
    resolved = _resolve(handler, enum_schema) or {}
    for union in ("oneOf", "anyOf"):
        alternatives = resolved.get(union)
        if alternatives and any(
            _flattened_alternative(a) is not a for a in alternatives
        ):
            enum_schema = {union: [_flattened_alternative(a) for a in alternatives]}
            break
    if optional:
        present_keys = [{"required": [key]} for key in keys]
        absent = {
            "not": present_keys[0]
            if len(present_keys) == 1
            else {"anyOf": present_keys}
        }
        enum_schema = {"anyOf": [enum_schema, absent]}
    outer: JsonSchemaValue = {"allOf": [enum_schema, parent]}
    for key in ("title", "description"):
        if key in parent:
            outer[key] = parent.pop(key)
    return outer
