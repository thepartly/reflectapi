"""`#[reflectapi(distinct)]` types in the generated Python client.

The type checks run with the adjacent `pyright-distinct.json`, after the
conformance fixture is generated: each `# type: ignore` marks an error pyright
must report, and an unneeded one is itself an error. The tests run with pytest.
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import assert_type
from uuid import UUID

sys.path.insert(0, str(Path(__file__).parents[2] / "target/conformance/python"))

from conformance_client.conformance.cases import (  # noqa: E402
    OrderNumber,
    SessionId,
    UserId,
    input,
    output,
)

RAW = UUID("67e55044-10b1-426f-9247-bb680e5fe0c8")


def check_distinct_ids_are_not_interchangeable(session: SessionId) -> None:
    user = UserId(RAW)
    input.Distinct(user=user, orders=[], tagged="", nickname=None)
    input.Distinct(user=RAW, orders=[], tagged="", nickname=None)  # type: ignore
    input.Distinct(user=session, orders=[], tagged="", nickname=None)  # type: ignore
    input.Distinct(
        user=user,
        orders=[7],  # type: ignore
        tagged="",
        nickname=None,
    )


def check_a_distinct_id_is_still_its_base(value: output.Distinct) -> None:
    assert_type(value.user, UserId)
    assert_type(value.orders, list[OrderNumber])
    as_uuid: UUID = value.user
    as_int: int = value.orders[0]
    assert as_uuid and as_int


def test_a_distinct_value_is_its_base_at_runtime():
    user = UserId(RAW)
    assert user == RAW
    assert type(user) is UUID
    model = input.Distinct(
        user=user, orders=[OrderNumber(7)], tagged="t", nickname=None
    )
    assert model.model_dump(mode="json", exclude_unset=True) == {
        "user": str(RAW),
        "orders": [7],
        "tagged": "t",
        "nickname": None,
    }


def test_a_response_reads_a_distinct_value_as_its_base():
    value = output.Distinct.model_validate(
        {
            "user": str(RAW),
            "session": None,
            "orders": [1, 2],
            "tagged": "t",
            "nickname": None,
        }
    )
    assert value.user == RAW
    assert value.orders == [1, 2]
