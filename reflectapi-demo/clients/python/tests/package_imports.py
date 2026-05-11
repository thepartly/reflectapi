"""Shared imports from the real generated demo package."""

# ruff: noqa: F401

from api_client import AsyncClient, Client
from api_client.myapi import HealthCheckFail as MyapiHealthCheckFail
from api_client.myapi.model import (
    Behavior as MyapiModelBehavior,
    BehaviorAggressiveVariant as MyapiModelBehaviorAggressiveVariant,
    BehaviorOtherVariant as MyapiModelBehaviorOtherVariant,
    Kind as MyapiModelKind,
    KindBird as MyapiModelKindBird,
    KindCat as MyapiModelKindCat,
    KindDog as MyapiModelKindDog,
)
from api_client.myapi.model.input import Pet as MyapiModelInputPet
from api_client.myapi.model.output import Pet as MyapiModelOutputPet
from api_client.myapi.proto import (
    Headers as MyapiProtoHeaders,
    InternalError as MyapiProtoInternalError,
    Paginated as MyapiProtoPaginated,
    PetsCreateError as MyapiProtoPetsCreateError,
    PetsCreateErrorInvalidIdentityVariant as MyapiProtoPetsCreateErrorInvalidIdentityVariant,
    PetsListError as MyapiProtoPetsListError,
    PetsListErrorInternal as MyapiProtoPetsListErrorInternal,
    PetsListErrorInvalidCursor as MyapiProtoPetsListErrorInvalidCursor,
    PetsListErrorUnauthorized as MyapiProtoPetsListErrorUnauthorized,
    PetsListRequest as MyapiProtoPetsListRequest,
    PetsRemoveError as MyapiProtoPetsRemoveError,
    PetsRemoveRequest as MyapiProtoPetsRemoveRequest,
    PetsUpdateError as MyapiProtoPetsUpdateError,
    PetsUpdateErrorValidationVariant as MyapiProtoPetsUpdateErrorValidationVariant,
    PetsUpdateRequest as MyapiProtoPetsUpdateRequest,
    ValidationA as MyapiProtoValidationA,
    ValidationError as MyapiProtoValidationError,
    ValidationErrorValidationAVariant as MyapiProtoValidationErrorValidationAVariant,
)

__all__ = [
    name
    for name in globals()
    if name == "AsyncClient" or name == "Client" or name.startswith("Myapi")
]
