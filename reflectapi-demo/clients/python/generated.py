from __future__ import annotations

import datetime
from typing import Any, Optional, TypeVar, Generic, Union, Annotated, Literal

from pydantic import (
    BaseModel,
    ConfigDict,
    Field,
    RootModel,
    model_validator,
    model_serializer,
)
from reflectapi_runtime import (
    AsyncClientBase,
    ClientBase,
    ApiResponse,
    ReflectapiOption,
)
T = TypeVar("T")

class Headers(BaseModel):
    authorization: str

class Paginated(BaseModel, Generic[T]):
    cursor: str | None = None
    items: list[T]

Outputu8 = Any

class OutputPet(BaseModel):
    age: int | None = None
    behaviors: list[Behavior] = None
    kind: Kind
    name: str
    updated_at: datetime.datetime[Utc]

class PetsCreateRequest(BaseModel):
    field_0: Pet

class PetsListRequest(BaseModel):
    cursor: str | None = None
    limit: int | None = None

class PetsRemoveRequest(BaseModel):
    name: str

class InputOptionSomeVariant(BaseModel):
    field_0: T = None

class InputOption(RootModel[Union[Literal["None"] | InputOptionSomeVariant | Literal["Undefined"]]]):
    @model_validator(mode="before")
    @classmethod
    def _validate(cls, data):
        return data
    @model_serializer
    def _serialize(self):
        return self.root

class PetsUpdateRequest(BaseModel):
    age: ReflectapiOption[int] = None
    behaviors: ReflectapiOption[list[Behavior]] = None
    kind: Kind | None = None
    name: str

Tuple0 = Any

class UnauthorizedError(BaseModel):
    field_0: Tuple0 = None

class OutputEmpty(BaseModel):
    pass

class Infallible(BaseModel):
    pass

Outputf64 = Any

class OutputBehaviorAggressiveVariant(BaseModel):
    field_0: float
    field_1: str

class OutputBehaviorOtherVariant(BaseModel):
    description: str
    notes: str = None

class OutputBehavior(RootModel[
    Union[OutputBehaviorAggressiveVariant | Literal["Calm"] | OutputBehaviorOtherVariant]
]):
    @model_validator(mode="before")
    @classmethod
    def _validate(cls, data):
        return data
    @model_serializer
    def _serialize(self):
        return self.root

class OutputKindDogVariant(BaseModel):
    breed: str

OutputKind = Annotated[OutputKindDogVariant, Field(discriminator="type")]

class PetsCreateErrorInvalidIdentityVariant(BaseModel):
    message: str

class PetsCreateError(RootModel[
    Union[
    Literal["Conflict"],
    PetsCreateErrorInvalidIdentityVariant,
    Literal["NotAuthorized"]
]
]):
    @model_validator(mode="before")
    @classmethod
    def _validate(cls, data):
        return data
    @model_serializer
    def _serialize(self):
        return self.root

class PetsListError(RootModel[Literal["InvalidCursor"] | Literal["Unauthorized"]]):
    @model_validator(mode="before")
    @classmethod
    def _validate(cls, data):
        return data
    @model_serializer
    def _serialize(self):
        return self.root

class PetsRemoveError(RootModel[Literal["NotAuthorized"] | Literal["NotFound"]]):
    @model_validator(mode="before")
    @classmethod
    def _validate(cls, data):
        return data
    @model_serializer
    def _serialize(self):
        return self.root

class PetsUpdateError(RootModel[Literal["NotAuthorized"] | Literal["NotFound"]]):
    @model_validator(mode="before")
    @classmethod
    def _validate(cls, data):
        return data
    @model_serializer
    def _serialize(self):
        return self.root

class OutputOptionSomeVariant(BaseModel):
    field_0: T = None

class OutputOption(RootModel[Literal["None"] | OutputOptionSomeVariant]):
    @model_validator(mode="before")
    @classmethod
    def _validate(cls, data):
        return data
    @model_serializer
    def _serialize(self):
        return self.root

OutputDateTime = Any

OutputString = Any

OutputVec = Any

class ApiClient(ClientBase):
    async def health_check(self) -> ApiResponse[None]:
        return self._request_async("GET", "")
    async def pets_create(self) -> ApiResponse[None]:
        return self._request_async("POST", "")
    async def pets_delete(self) -> ApiResponse[None]:
        return self._request_async("POST", "")
    async def pets_get_first(self) -> ApiResponse[None]:
        return self._request_async("POST", "")
    async def pets_list(self) -> ApiResponse[None]:
        return self._request_async("GET", "")
    async def pets_remove(self) -> ApiResponse[None]:
        return self._request_async("POST", "")
    async def pets_update(self) -> ApiResponse[None]:
        return self._request_async("POST", "")
