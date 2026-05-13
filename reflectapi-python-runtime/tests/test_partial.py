"""Tests for ``ReflectapiPartialModel``.

Contract:

* A field that was never explicitly set on the instance is omitted
  from the serialised payload entirely.
* A field set to ``None`` is emitted as ``null``.
* A field set to a value is emitted as that value.
* Round-trip preserves the distinction via ``model_fields_set``.
"""

from __future__ import annotations

import json

import pytest
from pydantic import BaseModel, ValidationError

from reflectapi_runtime import ReflectapiPartialModel


class Snapshot(BaseModel):
    """Plain Pydantic model used as an inner type to exercise nested validation."""

    name: str
    description: str | None = None


class Item(ReflectapiPartialModel):
    identity: str
    snapshot: Snapshot | None = None
    age: int | None = None


class PartialSnapshot(ReflectapiPartialModel):
    """Partial inner type — its own absent/null distinction must survive."""

    name: str
    description: str | None = None


class PartialItem(ReflectapiPartialModel):
    identity: str
    snapshot: PartialSnapshot | None = None


class TestModelFieldsSetTracking:
    """``model_fields_set`` must reflect what was on the wire."""

    def test_absent_key_not_in_fields_set(self):
        item = Item.model_validate({"identity": "x"})
        assert item.snapshot is None
        assert "snapshot" not in item.model_fields_set

    def test_explicit_null_in_fields_set(self):
        item = Item.model_validate({"identity": "x", "snapshot": None})
        assert item.snapshot is None
        assert "snapshot" in item.model_fields_set

    def test_value_in_fields_set(self):
        item = Item.model_validate(
            {"identity": "x", "snapshot": {"name": "n"}}
        )
        assert isinstance(item.snapshot, Snapshot)
        assert "snapshot" in item.model_fields_set

    def test_kwargs_construction_only_marks_passed_keys(self):
        item = Item(identity="x", age=4)
        assert item.model_fields_set == {"identity", "age"}
        assert "snapshot" not in item.model_fields_set


class TestWireFormat:
    """Serialisation must omit unset fields, keep explicit nulls, and emit values."""

    def test_unset_field_omitted_on_wire(self):
        item = Item(identity="x")
        assert item.model_dump_json() == '{"identity":"x"}'

    def test_explicit_null_appears_on_wire(self):
        item = Item(identity="x", snapshot=None)
        payload = json.loads(item.model_dump_json())
        assert payload == {"identity": "x", "snapshot": None}

    def test_value_appears_on_wire(self):
        item = Item.model_validate(
            {"identity": "x", "snapshot": {"name": "n"}, "age": 4}
        )
        payload = json.loads(item.model_dump_json())
        assert payload == {
            "identity": "x",
            "snapshot": {"name": "n", "description": None},
            "age": 4,
        }

    def test_post_construction_assignment_lands_on_wire(self):
        """Post-construction assignment marks the field as set.

        Requires ``validate_assignment=True`` in ``model_config`` —
        generated client classes always include it, so attribute
        writes after construction land on the wire as expected.
        """

        class WithAssign(ReflectapiPartialModel):
            model_config = {"validate_assignment": True}
            name: str
            note: str | None = None

        m = WithAssign(name="x")
        assert m.model_dump_json() == '{"name":"x"}'
        m.note = None
        assert m.model_dump_json() == '{"name":"x","note":null}'


class TestNestedPartial:
    """A partial model inside a partial model preserves its own absent/null distinction."""

    def test_nested_absent_field_omitted(self):
        item = PartialItem.model_validate(
            {"identity": "x", "snapshot": {"name": "n"}}
        )
        # description was absent in the nested wire dict — must stay absent.
        assert item.model_dump_json() == (
            '{"identity":"x","snapshot":{"name":"n"}}'
        )

    def test_nested_explicit_null_emitted(self):
        item = PartialItem.model_validate(
            {"identity": "x", "snapshot": {"name": "n", "description": None}}
        )
        assert item.model_dump_json() == (
            '{"identity":"x","snapshot":{"name":"n","description":null}}'
        )


class TestInnerTypeValidation:
    """Pydantic validates inner types normally — no custom wrapper to bypass."""

    def test_inner_dict_becomes_typed_instance(self):
        item = Item.model_validate(
            {"identity": "x", "snapshot": {"name": "Bumper"}}
        )
        assert isinstance(item.snapshot, Snapshot)
        assert item.snapshot.name == "Bumper"

    def test_garbage_inner_data_rejected(self):
        with pytest.raises(ValidationError) as exc_info:
            Item.model_validate(
                {"identity": "x", "snapshot": {"wrong_field": True}}
            )
        # The error path includes 'snapshot.name', proving the inner
        # type's validator ran on the wire payload.
        loc = exc_info.value.errors()[0]["loc"]
        assert "snapshot" in loc and "name" in loc

    def test_container_of_models_is_validated(self):
        class Bag(ReflectapiPartialModel):
            items: list[Snapshot] | None = None

        bag = Bag.model_validate({"items": [{"name": "a"}, {"name": "b"}]})
        assert all(isinstance(s, Snapshot) for s in bag.items)
        assert [s.name for s in bag.items] == ["a", "b"]


class TestRoundTrip:
    """JSON round-trip must preserve which fields were on the wire."""

    def test_absent_field_stays_absent(self):
        item = Item.model_validate({"identity": "x"})
        reloaded = Item.model_validate_json(item.model_dump_json())
        assert "snapshot" not in reloaded.model_fields_set
        assert reloaded.model_dump_json() == '{"identity":"x"}'

    def test_explicit_null_stays_null(self):
        item = Item.model_validate({"identity": "x", "snapshot": None})
        reloaded = Item.model_validate_json(item.model_dump_json())
        assert "snapshot" in reloaded.model_fields_set
        assert reloaded.snapshot is None
        assert json.loads(reloaded.model_dump_json()) == {
            "identity": "x",
            "snapshot": None,
        }

    def test_value_round_trips_as_typed_instance(self):
        item = Item.model_validate(
            {"identity": "x", "snapshot": {"name": "n"}}
        )
        reloaded = Item.model_validate_json(item.model_dump_json())
        assert isinstance(reloaded.snapshot, Snapshot)


class TestEdgeCases:
    def test_field_with_default_value_omitted_when_not_provided(self):
        """A field whose declared default *isn't* ``None`` is still omitted when unset."""

        class M(ReflectapiPartialModel):
            count: int = 0

        # Default applies (count == 0) but wasn't on the wire.
        m = M()
        assert m.count == 0
        assert m.model_dump_json() == "{}"

    def test_field_provided_at_default_value_appears_on_wire(self):
        class M(ReflectapiPartialModel):
            count: int = 0

        m = M(count=0)
        assert m.model_dump_json() == '{"count":0}'

    def test_alias_respected_on_serialise(self):
        from pydantic import Field

        class M(ReflectapiPartialModel):
            class_: str | None = Field(default=None, alias="class")

        m = M.model_validate({"class": "x"})
        assert m.model_dump_json(by_alias=True) == '{"class":"x"}'

    def test_unset_alias_omitted_on_serialise(self):
        from pydantic import Field

        class M(ReflectapiPartialModel):
            class_: str | None = Field(default=None, alias="class")

        assert M().model_dump_json(by_alias=True) == "{}"

    def test_model_dump_python_obeys_same_rule(self):
        """`model_dump()` (Python dict) must also drop unset keys."""
        item = Item(identity="x")
        assert item.model_dump() == {"identity": "x"}
