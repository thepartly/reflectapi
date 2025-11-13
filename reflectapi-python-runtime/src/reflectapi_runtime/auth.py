"""Authentication handlers for ReflectAPI Python clients."""

from __future__ import annotations

import asyncio
import time
from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Any, Callable

import httpx


@dataclass
class AuthToken:
    """Represents an authentication token with metadata."""

    access_token: str
    token_type: str = "Bearer"
    expires_in: int | None = None
    refresh_token: str | None = None
    scope: str | None = None

    # Internal tracking
    _created_at: float | None = None

    def __post_init__(self):
        """Initialize creation timestamp."""
        if self._created_at is None:
            self._created_at = time.time()

    @property
    def is_expired(self) -> bool:
        """Check if the token is expired."""
        if self.expires_in is None:
            return False

        elapsed = time.time() - self._created_at
        # Add 60 second buffer for clock skew and network latency
        return elapsed >= (self.expires_in - 60)

    @property
    def expires_at(self) -> float | None:
        """Get the absolute expiration timestamp."""
        if self.expires_in is None:
            return None
        return self._created_at + self.expires_in

    def to_header_value(self) -> str:
        """Generate the Authorization header value."""
        return f"{self.token_type} {self.access_token}"


class AuthHandler(httpx.Auth):
    """Base class for authentication handlers that integrates directly with httpx."""

    @abstractmethod
    def apply_auth(self, request: httpx.Request) -> httpx.Request:
        """Apply authentication to a synchronous request."""
        pass

    @abstractmethod
    async def apply_auth_async(self, request: httpx.Request) -> httpx.Request:
        """Apply authentication to an asynchronous request."""
        pass

    def auth_flow(self, request: httpx.Request):
        """httpx sync auth flow implementation."""
        yield self.apply_auth(request)

    async def async_auth_flow(self, request: httpx.Request):
        """httpx async auth flow implementation."""
        yield await self.apply_auth_async(request)

    def __call__(self, request: httpx.Request) -> httpx.Request:
        """Allow handler to be used as a callable for httpx auth parameter."""
        return self.apply_auth(request)


class BearerTokenAuth(AuthHandler):
    """Simple Bearer token authentication handler."""

    def __init__(self, token: str):
        """Initialize with a bearer token.

        Args:
            token: The bearer token string
        """
        self.token = token

    def apply_auth(self, request: httpx.Request) -> httpx.Request:
        """Apply Bearer token to request headers."""
        request.headers["Authorization"] = f"Bearer {self.token}"
        return request

    async def apply_auth_async(self, request: httpx.Request) -> httpx.Request:
        """Apply Bearer token to async request headers."""
        return self.apply_auth(request)


class APIKeyAuth(AuthHandler):
    """API Key authentication handler with flexible placement options."""

    def __init__(
        self,
        api_key: str,
        header_name: str = "X-API-Key",
        param_name: str | None = None,
    ):
        """Initialize with API key and placement options.

        Args:
            api_key: The API key string
            header_name: Header name for the API key (default: X-API-Key)
            param_name: If provided, add API key as query parameter instead of header
        """
        self.api_key = api_key
        self.header_name = header_name
        self.param_name = param_name

    def apply_auth(self, request: httpx.Request) -> httpx.Request:
        """Apply API key to request."""
        if self.param_name:
            # Add as query parameter
            url = request.url
            params = dict(url.params)
            params[self.param_name] = self.api_key
            request.url = url.copy_with(params=params)
        else:
            # Add as header
            request.headers[self.header_name] = self.api_key
        return request

    async def apply_auth_async(self, request: httpx.Request) -> httpx.Request:
        """Apply API key to async request."""
        return self.apply_auth(request)


class BasicAuth(AuthHandler):
    """HTTP Basic authentication handler."""

    def __init__(self, username: str, password: str):
        """Initialize with username and password.

        Args:
            username: Username for basic auth
            password: Password for basic auth
        """
        self.auth = httpx.BasicAuth(username, password)

    def apply_auth(self, request: httpx.Request) -> httpx.Request:
        """Apply basic authentication."""
        # httpx.BasicAuth.auth_flow returns a generator, so we need to get the first (and only) result
        auth_flow = self.auth.auth_flow(request)
        return next(auth_flow)

    async def apply_auth_async(self, request: httpx.Request) -> httpx.Request:
        """Apply basic authentication to async request."""
        return self.apply_auth(request)


class OAuth2ClientCredentialsAuth(AuthHandler):
    """OAuth2 Client Credentials flow authentication handler."""

    def __init__(
        self,
        token_url: str,
        client_id: str,
        client_secret: str,
        scope: str | None = None,
        client: httpx.Client | httpx.AsyncClient | None = None,
    ):
        """Initialize OAuth2 client credentials authentication.

        Args:
            token_url: OAuth2 token endpoint URL
            client_id: OAuth2 client ID
            client_secret: OAuth2 client secret
            scope: Optional scope for the token request
            client: Optional HTTP client for token requests (will create if not provided)
        """
        self.token_url = token_url
        self.client_id = client_id
        self.client_secret = client_secret
        self.scope = scope

        # Token storage
        self._token: AuthToken | None = None
        self._token_lock = asyncio.Lock()

        # HTTP clients for token requests
        self._sync_client = (
            client if isinstance(client, httpx.Client) else httpx.Client()
        )
        self._async_client = (
            client if isinstance(client, httpx.AsyncClient) else httpx.AsyncClient()
        )
        self._owns_clients = client is None

    def _prepare_token_request(self) -> dict[str, Any]:
        """Prepare the token request data."""
        data = {
            "grant_type": "client_credentials",
            "client_id": self.client_id,
            "client_secret": self.client_secret,
        }

        if self.scope:
            data["scope"] = self.scope

        return data

    def _get_valid_token_sync(self) -> AuthToken:
        """Get a valid token, refreshing if necessary (synchronous)."""
        if self._token and not self._token.is_expired:
            return self._token

        # Request new token
        data = self._prepare_token_request()
        response = self._sync_client.post(
            self.token_url,
            data=data,
            headers={"Content-Type": "application/x-www-form-urlencoded"},
        )
        response.raise_for_status()

        token_data = response.json()
        self._token = AuthToken(
            access_token=token_data["access_token"],
            token_type=token_data.get("token_type", "Bearer"),
            expires_in=token_data.get("expires_in"),
            refresh_token=token_data.get("refresh_token"),
            scope=token_data.get("scope"),
        )

        return self._token

    async def _get_valid_token_async(self) -> AuthToken:
        """Get a valid token, refreshing if necessary (asynchronous)."""
        async with self._token_lock:
            if self._token and not self._token.is_expired:
                return self._token

            # Request new token
            data = self._prepare_token_request()
            response = await self._async_client.post(
                self.token_url,
                data=data,
                headers={"Content-Type": "application/x-www-form-urlencoded"},
            )
            response.raise_for_status()

            token_data = response.json()
            self._token = AuthToken(
                access_token=token_data["access_token"],
                token_type=token_data.get("token_type", "Bearer"),
                expires_in=token_data.get("expires_in"),
                refresh_token=token_data.get("refresh_token"),
                scope=token_data.get("scope"),
            )

            return self._token

    def apply_auth(self, request: httpx.Request) -> httpx.Request:
        """Apply OAuth2 authentication to synchronous request."""
        token = self._get_valid_token_sync()
        request.headers["Authorization"] = token.to_header_value()
        return request

    async def apply_auth_async(self, request: httpx.Request) -> httpx.Request:
        """Apply OAuth2 authentication to asynchronous request."""
        token = await self._get_valid_token_async()
        request.headers["Authorization"] = token.to_header_value()
        return request

    def close(self) -> None:
        """Close HTTP clients if we own them."""
        if self._owns_clients:
            self._sync_client.close()

    async def aclose(self) -> None:
        """Close async HTTP clients if we own them."""
        if self._owns_clients:
            await self._async_client.aclose()


class OAuth2AuthorizationCodeAuth(AuthHandler):
    """OAuth2 Authorization Code flow authentication handler."""

    def __init__(
        self,
        access_token: str,
        refresh_token: str | None = None,
        token_url: str | None = None,
        client_id: str | None = None,
        client_secret: str | None = None,
        expires_in: int | None = None,
        client: httpx.Client | httpx.AsyncClient | None = None,
    ):
        """Initialize OAuth2 authorization code authentication.

        Args:
            access_token: Initial access token
            refresh_token: Refresh token for token renewal
            token_url: Token endpoint URL (required if refresh_token provided)
            client_id: OAuth2 client ID (required if refresh_token provided)
            client_secret: OAuth2 client secret (required if refresh_token provided)
            expires_in: Token expiration time in seconds
            client: Optional HTTP client for token refresh requests
        """
        self._token = AuthToken(
            access_token=access_token,
            refresh_token=refresh_token,
            expires_in=expires_in,
        )

        self.token_url = token_url
        self.client_id = client_id
        self.client_secret = client_secret
        self._token_lock = asyncio.Lock()

        # Validate refresh configuration
        if refresh_token and not all([token_url, client_id, client_secret]):
            raise ValueError(
                "token_url, client_id, and client_secret are required when refresh_token is provided"
            )

        # HTTP clients for token refresh
        self._sync_client = (
            client if isinstance(client, httpx.Client) else httpx.Client()
        )
        self._async_client = (
            client if isinstance(client, httpx.AsyncClient) else httpx.AsyncClient()
        )
        self._owns_clients = client is None

    def _refresh_token_sync(self) -> AuthToken:
        """Refresh the access token (synchronous)."""
        if not self._token.refresh_token:
            raise RuntimeError("No refresh token available")

        data = {
            "grant_type": "refresh_token",
            "refresh_token": self._token.refresh_token,
            "client_id": self.client_id,
            "client_secret": self.client_secret,
        }

        response = self._sync_client.post(
            self.token_url,
            data=data,
            headers={"Content-Type": "application/x-www-form-urlencoded"},
        )
        response.raise_for_status()

        token_data = response.json()
        self._token = AuthToken(
            access_token=token_data["access_token"],
            token_type=token_data.get("token_type", "Bearer"),
            expires_in=token_data.get("expires_in"),
            refresh_token=token_data.get("refresh_token", self._token.refresh_token),
            scope=token_data.get("scope"),
        )

        return self._token

    async def _refresh_token_async(self) -> AuthToken:
        """Refresh the access token (asynchronous)."""
        if not self._token.refresh_token:
            raise RuntimeError("No refresh token available")

        data = {
            "grant_type": "refresh_token",
            "refresh_token": self._token.refresh_token,
            "client_id": self.client_id,
            "client_secret": self.client_secret,
        }

        response = await self._async_client.post(
            self.token_url,
            data=data,
            headers={"Content-Type": "application/x-www-form-urlencoded"},
        )
        response.raise_for_status()

        token_data = response.json()
        self._token = AuthToken(
            access_token=token_data["access_token"],
            token_type=token_data.get("token_type", "Bearer"),
            expires_in=token_data.get("expires_in"),
            refresh_token=token_data.get("refresh_token", self._token.refresh_token),
            scope=token_data.get("scope"),
        )

        return self._token

    def _get_valid_token_sync(self) -> AuthToken:
        """Get a valid token, refreshing if necessary (synchronous)."""
        if not self._token.is_expired:
            return self._token

        if self._token.refresh_token:
            return self._refresh_token_sync()

        raise RuntimeError("Access token is expired and no refresh token available")

    async def _get_valid_token_async(self) -> AuthToken:
        """Get a valid token, refreshing if necessary (asynchronous)."""
        async with self._token_lock:
            if not self._token.is_expired:
                return self._token

            if self._token.refresh_token:
                return await self._refresh_token_async()

            raise RuntimeError("Access token is expired and no refresh token available")

    def apply_auth(self, request: httpx.Request) -> httpx.Request:
        """Apply OAuth2 authentication to synchronous request."""
        token = self._get_valid_token_sync()
        request.headers["Authorization"] = token.to_header_value()
        return request

    async def apply_auth_async(self, request: httpx.Request) -> httpx.Request:
        """Apply OAuth2 authentication to asynchronous request."""
        token = await self._get_valid_token_async()
        request.headers["Authorization"] = token.to_header_value()
        return request

    @property
    def access_token(self) -> str:
        """Get the current access token."""
        return self._token.access_token

    @property
    def is_expired(self) -> bool:
        """Check if the current token is expired."""
        return self._token.is_expired

    def close(self) -> None:
        """Close HTTP clients if we own them."""
        if self._owns_clients:
            self._sync_client.close()

    async def aclose(self) -> None:
        """Close async HTTP clients if we own them."""
        if self._owns_clients:
            await self._async_client.aclose()


class CustomAuth(AuthHandler):
    """Custom authentication handler using user-provided functions."""

    def __init__(
        self,
        sync_auth_func: Callable[[httpx.Request], httpx.Request] | None = None,
        async_auth_func: Callable[[httpx.Request], httpx.Request] | None = None,
    ):
        """Initialize with custom authentication functions.

        Args:
            sync_auth_func: Synchronous function that takes httpx.Request and returns httpx.Request
            async_auth_func: Asynchronous function that takes httpx.Request and returns httpx.Request
        """
        if not sync_auth_func and not async_auth_func:
            raise ValueError(
                "At least one of sync_auth_func or async_auth_func must be provided"
            )

        self.sync_auth_func = sync_auth_func
        self.async_auth_func = async_auth_func

    def apply_auth(self, request: httpx.Request) -> httpx.Request:
        """Apply custom synchronous authentication."""
        if not self.sync_auth_func:
            raise RuntimeError("No synchronous auth function provided")
        return self.sync_auth_func(request)

    async def apply_auth_async(self, request: httpx.Request) -> httpx.Request:
        """Apply custom asynchronous authentication."""
        if not self.async_auth_func:
            raise RuntimeError("No asynchronous auth function provided")
        return await self.async_auth_func(request)


# Convenience factory functions
def bearer_token(token: str) -> BearerTokenAuth:
    """Create a Bearer token authentication handler."""
    return BearerTokenAuth(token)


def api_key(
    key: str, header_name: str = "X-API-Key", param_name: str | None = None
) -> APIKeyAuth:
    """Create an API key authentication handler."""
    return APIKeyAuth(key, header_name, param_name)


def basic_auth(username: str, password: str) -> BasicAuth:
    """Create a Basic authentication handler."""
    return BasicAuth(username, password)


def oauth2_client_credentials(
    token_url: str, client_id: str, client_secret: str, scope: str | None = None
) -> OAuth2ClientCredentialsAuth:
    """Create an OAuth2 client credentials authentication handler."""
    return OAuth2ClientCredentialsAuth(token_url, client_id, client_secret, scope)


def oauth2_authorization_code(
    access_token: str,
    refresh_token: str | None = None,
    token_url: str | None = None,
    client_id: str | None = None,
    client_secret: str | None = None,
    expires_in: int | None = None,
) -> OAuth2AuthorizationCodeAuth:
    """Create an OAuth2 authorization code authentication handler."""
    return OAuth2AuthorizationCodeAuth(
        access_token, refresh_token, token_url, client_id, client_secret, expires_in
    )
