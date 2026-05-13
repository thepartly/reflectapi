"""ReflectAPI Python Runtime Library.

This package provides the core runtime components for ReflectAPI-generated clients.
"""

from .auth import (
    APIKeyAuth,
    AuthHandler,
    AuthToken,
    BasicAuth,
    BearerTokenAuth,
    CustomAuth,
    OAuth2AuthorizationCodeAuth,
    OAuth2ClientCredentialsAuth,
    api_key,
    basic_auth,
    bearer_token,
    oauth2_authorization_code,
    oauth2_client_credentials,
)
from .batch import BatchClient
from .client import AsyncClientBase, ClientBase
from .duration import ReflectapiDuration
from .transport import AsyncClient, Client, Request, Response
from .exceptions import (
    ApiError,
    ApplicationError,
    NetworkError,
    TimeoutError,
    ValidationError,
)
from .hypothesis_strategies import (
    HAS_HYPOTHESIS,
    api_model_strategy,
    enhanced_strategy_for_type,
    strategy_for_pydantic_model,
    strategy_for_type,
)
from .middleware import AsyncMiddleware
from .partial import ReflectapiPartialModel
from .response import ApiResponse, TransportMetadata
from .serde import parse_externally_tagged, serialize_externally_tagged
from .streaming import AsyncStreamingClient, StreamingResponse
from .testing import (
    AsyncCassetteMiddleware,
    CassetteClient,
    CassetteMiddleware,
    MockClient,
    TestClientMixin,
)
from .types import BatchResult, ReflectapiEmpty, ReflectapiInfallible

__version__ = "0.17.3a1"

__all__ = [
    # Authentication
    "APIKeyAuth",
    "AuthHandler",
    "AuthToken",
    "BasicAuth",
    "BearerTokenAuth",
    "CustomAuth",
    "OAuth2AuthorizationCodeAuth",
    "OAuth2ClientCredentialsAuth",
    "api_key",
    "basic_auth",
    "bearer_token",
    "oauth2_authorization_code",
    "oauth2_client_credentials",
    # Core
    "ApiError",
    "ApiResponse",
    "ApplicationError",
    "AsyncCassetteMiddleware",
    "AsyncClientBase",
    "AsyncClient",
    "AsyncStreamingClient",
    "BatchClient",
    "BatchResult",
    "CassetteClient",
    "CassetteMiddleware",
    "Client",
    "ClientBase",
    "Request",
    "Response",
    "HAS_HYPOTHESIS",
    "AsyncMiddleware",
    "MockClient",
    "NetworkError",
    "ReflectapiDuration",
    "ReflectapiEmpty",
    "ReflectapiInfallible",
    "ReflectapiPartialModel",
    "parse_externally_tagged",
    "serialize_externally_tagged",
    "StreamingResponse",
    "TestClientMixin",
    "TimeoutError",
    "TransportMetadata",
    "ValidationError",
    "api_model_strategy",
    "enhanced_strategy_for_type",
    "strategy_for_pydantic_model",
    "strategy_for_type",
]
