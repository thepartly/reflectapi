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
from .middleware import Middleware
from .option import (
    Option,
    ReflectapiOption,
    Undefined,
    none,
    serialize_option_dict,
    some,
    undefined,
)
from .response import ApiResponse, TransportMetadata
from .streaming import AsyncStreamingClient, StreamingResponse
from .testing import (
    AsyncCassetteMiddleware,
    CassetteClient,
    CassetteMiddleware,
    MockClient,
    TestClientMixin,
)
from .types import BatchResult, ReflectapiEmpty, ReflectapiInfallible

__version__ = "0.1.0"

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
    "AsyncStreamingClient",
    "BatchClient",
    "BatchResult",
    "CassetteClient",
    "CassetteMiddleware",
    "ClientBase",
    "HAS_HYPOTHESIS",
    "Middleware",
    "MockClient",
    "NetworkError",
    "Option",
    "ReflectapiEmpty",
    "ReflectapiInfallible",
    "ReflectapiOption",
    "StreamingResponse",
    "TestClientMixin",
    "TimeoutError",
    "TransportMetadata",
    "Undefined",
    "ValidationError",
    "api_model_strategy",
    "enhanced_strategy_for_type",
    "none",
    "serialize_option_dict",
    "some",
    "strategy_for_pydantic_model",
    "strategy_for_type",
    "undefined",
]
