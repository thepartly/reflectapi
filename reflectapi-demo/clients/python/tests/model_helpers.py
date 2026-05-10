"""Helpers that construct generated models through the real module API."""

from myapi.model import (
    Behavior,
    BehaviorAggressiveVariant,
    BehaviorOtherVariant,
)
from myapi.proto import (
    PetsCreateError,
    PetsCreateErrorInvalidIdentityVariant,
)


def root_value(value):
    return getattr(value, "root", value)


def calm_behavior() -> Behavior:
    return Behavior(root="Calm")


def aggressive_behavior(level: float, description: str) -> Behavior:
    return Behavior(
        root=BehaviorAggressiveVariant(field_0=level, field_1=description)
    )


def other_behavior(description: str, notes: str | None = None) -> Behavior:
    return Behavior(root=BehaviorOtherVariant(description=description, notes=notes))


def conflict_create_error() -> PetsCreateError:
    return PetsCreateError(root="Conflict")


def not_authorized_create_error() -> PetsCreateError:
    return PetsCreateError(root="NotAuthorized")


def invalid_identity_create_error(message: str) -> PetsCreateError:
    return PetsCreateError(
        root=PetsCreateErrorInvalidIdentityVariant(message=message)
    )
