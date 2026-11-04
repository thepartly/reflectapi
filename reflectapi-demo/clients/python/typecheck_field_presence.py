# pyright: reportUnnecessaryTypeIgnoreComment=error
"""Checked by pyright in CI, never run.

`behaviors` may be omitted but is never null on the wire, so passing None must
be a type error rather than only a validation error at runtime. If the
generated annotation admits None again, the ignore becomes unnecessary and
`reportUnnecessaryTypeIgnoreComment` fails the check.
"""

from datetime import datetime

from api_client.myapi.model import Kind
from api_client.myapi.model.input import Pet as InputPet
from api_client.myapi.model.output import Pet as OutputPet


def omitted_lists_reject_none(kind: Kind, now: datetime) -> None:
    InputPet(name="a", kind=kind)
    InputPet(name="a", kind=kind, behaviors=None)  # pyright: ignore[reportArgumentType]
    OutputPet(name="a", kind=kind, updated_at=now)
    OutputPet(name="a", kind=kind, updated_at=now, behaviors=None)  # pyright: ignore[reportArgumentType]


def omitted_lists_read_as_lists(pet: OutputPet) -> int:
    return len(pet.behaviors)
