"""ReflectapiDuration: a timedelta written back exactly as it was read."""

import copy
import json
import pickle
import typing
from datetime import timedelta

import pydantic
import pytest
from pydantic import BaseModel, RootModel, TypeAdapter

from reflectapi_runtime import ReflectapiDuration, WireDuration
from reflectapi_runtime.client import _json_serializer


class Timed(BaseModel):
    elapsed: ReflectapiDuration
    maybe: ReflectapiDuration | None = None


NANOS = {"elapsed": {"secs": 1, "nanos": 123_456_789}}


def roundtrip(data: dict) -> dict:
    model = Timed.model_validate_json(json.dumps(data))
    return json.loads(model.model_dump_json(exclude_unset=True))


def test_nanoseconds_round_trip_exactly():
    assert roundtrip(NANOS) == NANOS


@pytest.mark.parametrize(
    "wire",
    [
        {"secs": 0, "nanos": 0},
        {"secs": 0, "nanos": 1},
        {"secs": 30, "nanos": 0},
        {"secs": 1, "nanos": 999_999_999},
        {"secs": 2**63, "nanos": 500},
    ],
)
def test_every_value_round_trips(wire):
    assert roundtrip({"elapsed": wire}) == {"elapsed": wire}


def test_it_is_a_timedelta_truncated_to_the_microsecond():
    model = Timed.model_validate(NANOS)
    assert isinstance(model.elapsed, timedelta)
    assert isinstance(model.elapsed, WireDuration)
    assert model.elapsed == timedelta(seconds=1, microseconds=123_456)
    assert model.elapsed.wire == (1, 123_456_789)
    # Truncated, not rounded: just under two seconds stays under two seconds.
    almost = Timed.model_validate({"elapsed": {"secs": 1, "nanos": 999_999_999}})
    assert almost.elapsed == timedelta(seconds=1, microseconds=999_999)


def test_a_duration_beyond_timedelta_reads_as_its_max():
    forever = {"secs": 2**64 - 1, "nanos": 999_999_999}
    model = Timed.model_validate({"elapsed": forever})
    assert model.elapsed == timedelta.max
    assert json.loads(model.model_dump_json(exclude_unset=True)) == {"elapsed": forever}


def test_a_new_value_is_written_as_whole_microseconds():
    model = Timed.model_validate(NANOS)
    longer = model.elapsed + timedelta(seconds=1)
    assert type(longer) is timedelta
    model.elapsed = longer
    assert json.loads(model.model_dump_json(exclude_unset=True)) == {
        "elapsed": {"secs": 2, "nanos": 123_456_000}
    }


def test_a_timedelta_from_user_code_is_written_as_before():
    model = Timed(elapsed=timedelta(hours=2, seconds=3, microseconds=4))
    assert not isinstance(model.elapsed, WireDuration)
    assert json.loads(model.model_dump_json()) == {
        "elapsed": {"secs": 7203, "nanos": 4_000},
        "maybe": None,
    }


def test_other_inputs_are_read_as_before():
    assert Timed.model_validate({"elapsed": 5}).elapsed == timedelta(seconds=5)
    iso = Timed.model_validate({"elapsed": "PT1.5S"})
    assert iso.elapsed == timedelta(seconds=1.5)
    floats = Timed.model_validate({"elapsed": {"secs": 1.5, "nanos": 0}})
    assert floats.elapsed == timedelta(seconds=1.5)
    assert not isinstance(floats.elapsed, WireDuration)


def test_a_value_passed_on_keeps_its_wire_value():
    first = Timed.model_validate(NANOS)
    second = Timed(elapsed=first.elapsed)
    assert json.loads(second.model_dump_json(exclude_unset=True)) == NANOS


def test_python_mode_dumps_a_timedelta():
    dumped = Timed.model_validate(NANOS).model_dump()
    assert dumped["elapsed"] == timedelta(seconds=1, microseconds=123_456)


def test_copies_and_pickles_keep_the_wire_value():
    model = Timed.model_validate(NANOS)
    for other in (
        copy.copy(model.elapsed),
        copy.deepcopy(model.elapsed),
        pickle.loads(pickle.dumps(model.elapsed)),
    ):
        assert isinstance(other, WireDuration)
        assert other.wire == (1, 123_456_789)
        assert other == model.elapsed
    copied = model.model_copy(deep=True)
    assert json.loads(copied.model_dump_json(exclude_unset=True)) == NANOS


def test_containers_and_bare_values_round_trip():
    class Many(BaseModel):
        each: list[ReflectapiDuration]

    many = {"each": [{"secs": 1, "nanos": 1}, {"secs": 0, "nanos": 999}]}
    model = Many.model_validate_json(json.dumps(many))
    assert json.loads(model.model_dump_json()) == many
    text = '{"secs":3,"nanos":7}'
    adapter = TypeAdapter(ReflectapiDuration)
    assert adapter.dump_json(adapter.validate_json(text)).decode() == text
    root = RootModel[ReflectapiDuration].model_validate_json(text)
    assert root.model_dump_json() == text


def test_invalid_input_is_rejected_as_a_timedelta_would_be():
    with pytest.raises(pydantic.ValidationError):
        Timed.model_validate({"elapsed": {"secs": "nope"}})
    with pytest.raises(pydantic.ValidationError):
        Timed.model_validate({"elapsed": "nope"})


def test_json_schema_reads_a_duration_and_writes_secs_and_nanos():
    validation = Timed.model_json_schema(mode="validation")
    assert validation["properties"]["elapsed"]["type"] == "string"
    serialization = Timed.model_json_schema(mode="serialization")
    elapsed = serialization["properties"]["elapsed"]
    assert elapsed["type"] == "object"
    assert set(elapsed["required"]) == {"secs", "nanos"}


def test_a_bare_request_body_keeps_the_wire_value():
    value = TypeAdapter(ReflectapiDuration).validate_python(NANOS["elapsed"])
    assert json.loads(json.dumps(value, default=_json_serializer)) == NANOS["elapsed"]
    plain = json.dumps(timedelta(milliseconds=1500), default=_json_serializer)
    assert json.loads(plain) == {"secs": 1, "nanos": 500_000_000}


def test_the_annotation_is_a_timedelta_for_type_checkers():
    assert typing.get_args(ReflectapiDuration)[0] is timedelta
