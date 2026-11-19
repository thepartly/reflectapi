"""ReflectapiDateTime: a datetime written back as the text it was read from (#243)."""

import copy
import enum
import json
import pickle
import typing
from datetime import UTC, datetime, timedelta

import pydantic
import pytest
from pydantic import BaseModel, RootModel, TypeAdapter

from reflectapi_runtime import ClientBase, ReflectapiDateTime, WireDateTime
from reflectapi_runtime.client import _json_serializer


class Stamped(BaseModel):
    at: ReflectapiDateTime
    local: ReflectapiDateTime
    maybe: ReflectapiDateTime | None = None


NANOS = {
    "at": "2026-11-19T10:20:30.123456789Z",
    "local": "2026-11-19T10:20:30.123456789",
}


def roundtrip(data: dict) -> dict:
    model = Stamped.model_validate_json(json.dumps(data))
    return json.loads(model.model_dump_json(exclude_unset=True))


def test_nanoseconds_round_trip_exactly():
    assert roundtrip(NANOS) == NANOS


@pytest.mark.parametrize(
    "text",
    [
        "2026-11-19T10:20:30Z",
        "2026-11-19T10:20:30+00:00",
        "2026-11-19T10:20:30.5+13:00",
        "2026-11-19T10:20:30.000000001Z",
    ],
)
def test_every_spelling_round_trips(text):
    assert roundtrip({"at": text, "local": "2026-11-19T10:20:30"})["at"] == text


def test_it_is_a_datetime_with_the_microseconds():
    model = Stamped.model_validate_json(json.dumps(NANOS))
    expected = datetime(2026, 11, 19, 10, 20, 30, 123456, tzinfo=UTC)
    assert isinstance(model.at, WireDateTime)
    assert model.at == expected
    assert hash(model.at) == hash(expected)
    assert isinstance(model.at, WireDateTime)
    assert model.at.wire == NANOS["at"]


def test_a_new_value_is_written_as_a_plain_datetime():
    model = Stamped.model_validate_json(json.dumps(NANOS))
    later = model.at + timedelta(seconds=1)
    replaced = model.local.replace(second=0)
    assert getattr(later, "wire", None) is None
    assert getattr(replaced, "wire", None) is None
    written = json.loads(Stamped(at=later, local=replaced).model_dump_json())
    assert written["at"] == "2026-11-19T10:20:31.123456Z"
    assert written["local"] == "2026-11-19T10:20:00.123456"


def test_a_datetime_from_user_code_is_written_as_before():
    model = Stamped(at=datetime(2026, 1, 1, tzinfo=UTC), local=datetime(2026, 1, 1))
    assert json.loads(model.model_dump_json(exclude_unset=True)) == {
        "at": "2026-01-01T00:00:00Z",
        "local": "2026-01-01T00:00:00",
    }


def test_a_value_passed_on_keeps_its_text():
    model = Stamped.model_validate_json(json.dumps(NANOS))
    again = Stamped(at=model.at, local=model.local)
    assert json.loads(again.model_dump_json(exclude_unset=True)) == NANOS


def test_python_mode_dumps_a_datetime():
    model = Stamped.model_validate_json(json.dumps(NANOS))
    assert isinstance(model.model_dump()["at"], datetime)


def test_copies_and_pickles_keep_the_text():
    model = Stamped.model_validate_json(json.dumps(NANOS))
    assert isinstance(model.at, WireDateTime)
    for other in (
        copy.copy(model.at),
        copy.deepcopy(model.at),
        pickle.loads(pickle.dumps(model.at)),
    ):
        assert other.wire == NANOS["at"]
        assert other == model.at
    copied = model.model_copy(deep=True)
    assert json.loads(copied.model_dump_json(exclude_unset=True)) == NANOS


def test_containers_and_bare_values_round_trip():
    class Many(BaseModel):
        ats: list[ReflectapiDateTime]

    many = {"ats": ["2026-11-19T10:20:30.123456789Z", "2026-11-19T10:20:30Z"]}
    model = Many.model_validate_json(json.dumps(many))
    assert json.loads(model.model_dump_json()) == many
    text = '"2026-11-19T10:20:30.999999999Z"'
    adapter = TypeAdapter(ReflectapiDateTime)
    assert adapter.dump_json(adapter.validate_json(text)).decode() == text
    root = RootModel[ReflectapiDateTime].model_validate_json(text)
    assert root.model_dump_json() == text


def test_invalid_input_is_rejected_as_a_datetime_would_be():
    with pytest.raises(pydantic.ValidationError) as error:
        Stamped.model_validate_json(
            json.dumps({"at": "nope", "local": "2026-11-19T10:20:30"})
        )
    assert "datetime" in str(error.value)


def test_json_schema_is_a_date_time_string():
    schema = Stamped.model_json_schema()
    assert schema["properties"]["at"] == {
        "format": "date-time",
        "title": "At",
        "type": "string",
    }


def test_a_bare_request_body_keeps_the_text():
    text = '"2026-11-19T10:20:30.123456789Z"'
    value = TypeAdapter(ReflectapiDateTime).validate_json(text)
    assert json.dumps(value, default=_json_serializer) == text


def test_the_annotation_is_a_datetime_for_type_checkers():
    # Any datetime can be assigned to a generated field.
    assert typing.get_args(ReflectapiDateTime)[0] is datetime
    model = Stamped(at=datetime.now(UTC), local=datetime.now())
    assert not isinstance(model.at, WireDateTime)


class Color(enum.Enum):
    RED = "red"


class Headers(BaseModel):
    version: ReflectapiDateTime
    color: Color | None = None


def test_a_header_keeps_its_text_and_enums_are_sent_as_values():
    version = TypeAdapter(ReflectapiDateTime).validate_python(NANOS["at"])
    headers = Headers(version=version, color=Color.RED)
    built = ClientBase("http://test")._build_headers({}, headers)
    assert built == {"version": NANOS["at"], "color": "red"}
    plain = Headers(version=datetime(2026, 1, 1, tzinfo=UTC))
    assert ClientBase("http://test")._build_headers({}, plain) == {
        "version": "2026-01-01T00:00:00Z"
    }
