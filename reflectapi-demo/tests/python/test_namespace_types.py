"""A field whose type is in another namespace has that type, not `Any`.

Checked with the adjacent strict pyrightconfig.json against the demo client,
whose `myapi.model.output` models name types in `myapi.model`.
"""

from __future__ import annotations

from typing import assert_type

from api_client.myapi import model
from api_client.myapi.model import output


def check_a_field_from_another_namespace(pet: output.Pet) -> None:
    assert_type(pet.kind, model.Kind)
