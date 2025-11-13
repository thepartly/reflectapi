"""Tests for authentication handlers."""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "src"))

import time
from unittest.mock import AsyncMock, Mock, patch

import httpx
import pytest
from pydantic import BaseModel

from reflectapi_runtime import AsyncClientBase, ClientBase
from reflectapi_runtime.auth import (
    APIKeyAuth,
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


class TestAuthToken:
    """Test AuthToken functionality."""

    def test_initialization(self):
        """Test token initialization with defaults."""
        token = AuthToken("test_token")

        assert token.access_token == "test_token"
        assert token.token_type == "Bearer"
        assert token.expires_in is None
        assert token.refresh_token is None
        assert token.scope is None
        assert token._created_at is not None

    def test_initialization_with_params(self):
        """Test token initialization with all parameters."""
        token = AuthToken(
            access_token="test_token",
            token_type="Custom",
            expires_in=3600,
            refresh_token="refresh_token",
            scope="read write",
        )

        assert token.access_token == "test_token"
        assert token.token_type == "Custom"
        assert token.expires_in == 3600
        assert token.refresh_token == "refresh_token"
        assert token.scope == "read write"

    def test_is_expired_no_expiry(self):
        """Test token without expiry is never expired."""
        token = AuthToken("test_token")
        assert not token.is_expired

    def test_is_expired_not_expired(self):
        """Test non-expired token."""
        token = AuthToken("test_token", expires_in=3600)
        assert not token.is_expired

    def test_is_expired_expired(self):
        """Test expired token."""
        with patch("time.time", return_value=1000):
            token = AuthToken("test_token", expires_in=60)

        with patch("time.time", return_value=1200):  # 200 seconds later
            assert token.is_expired

    def test_is_expired_with_buffer(self):
        """Test token expiry includes 60 second buffer."""
        with patch("time.time", return_value=1000):
            token = AuthToken("test_token", expires_in=120)

        # 70 seconds later (within buffer) - should be expired
        with patch("time.time", return_value=1070):
            assert token.is_expired

    def test_expires_at_no_expiry(self):
        """Test expires_at when no expiry set."""
        token = AuthToken("test_token")
        assert token.expires_at is None

    def test_expires_at_with_expiry(self):
        """Test expires_at calculation."""
        with patch("time.time", return_value=1000):
            token = AuthToken("test_token", expires_in=3600)

        assert token.expires_at == 4600

    def test_to_header_value_default(self):
        """Test header value generation with default token type."""
        token = AuthToken("test_token")
        assert token.to_header_value() == "Bearer test_token"

    def test_to_header_value_custom(self):
        """Test header value generation with custom token type."""
        token = AuthToken("test_token", token_type="Custom")
        assert token.to_header_value() == "Custom test_token"


class TestBearerTokenAuth:
    """Test BearerTokenAuth handler."""

    def test_apply_auth(self):
        """Test applying bearer token authentication."""
        auth = BearerTokenAuth("test_token")
        request = httpx.Request("GET", "http://example.com")

        authenticated_request = auth.apply_auth(request)

        assert authenticated_request.headers["Authorization"] == "Bearer test_token"

    @pytest.mark.asyncio
    async def test_apply_auth_async(self):
        """Test applying bearer token authentication asynchronously."""
        auth = BearerTokenAuth("test_token")
        request = httpx.Request("GET", "http://example.com")

        authenticated_request = await auth.apply_auth_async(request)

        assert authenticated_request.headers["Authorization"] == "Bearer test_token"

    def test_callable_interface(self):
        """Test that auth handler can be used as callable."""
        auth = BearerTokenAuth("test_token")
        request = httpx.Request("GET", "http://example.com")

        authenticated_request = auth(request)

        assert authenticated_request.headers["Authorization"] == "Bearer test_token"


class TestAPIKeyAuth:
    """Test APIKeyAuth handler."""

    def test_apply_auth_header(self):
        """Test applying API key as header."""
        auth = APIKeyAuth("test_key")
        request = httpx.Request("GET", "http://example.com")

        authenticated_request = auth.apply_auth(request)

        assert authenticated_request.headers["X-API-Key"] == "test_key"

    def test_apply_auth_custom_header(self):
        """Test applying API key with custom header name."""
        auth = APIKeyAuth("test_key", header_name="Authorization")
        request = httpx.Request("GET", "http://example.com")

        authenticated_request = auth.apply_auth(request)

        assert authenticated_request.headers["Authorization"] == "test_key"

    def test_apply_auth_query_param(self):
        """Test applying API key as query parameter."""
        auth = APIKeyAuth("test_key", param_name="api_key")
        request = httpx.Request("GET", "http://example.com")

        authenticated_request = auth.apply_auth(request)

        assert "api_key=test_key" in str(authenticated_request.url)

    def test_apply_auth_query_param_existing_params(self):
        """Test applying API key as query parameter with existing params."""
        auth = APIKeyAuth("test_key", param_name="api_key")
        request = httpx.Request("GET", "http://example.com?existing=value")

        authenticated_request = auth.apply_auth(request)

        url_str = str(authenticated_request.url)
        assert "api_key=test_key" in url_str
        assert "existing=value" in url_str

    @pytest.mark.asyncio
    async def test_apply_auth_async(self):
        """Test applying API key authentication asynchronously."""
        auth = APIKeyAuth("test_key")
        request = httpx.Request("GET", "http://example.com")

        authenticated_request = await auth.apply_auth_async(request)

        assert authenticated_request.headers["X-API-Key"] == "test_key"


class TestBasicAuth:
    """Test BasicAuth handler."""

    def test_apply_auth(self):
        """Test applying basic authentication."""
        auth = BasicAuth("username", "password")
        request = httpx.Request("GET", "http://example.com")

        authenticated_request = auth.apply_auth(request)

        # httpx.BasicAuth encodes the credentials
        assert "Authorization" in authenticated_request.headers
        assert authenticated_request.headers["Authorization"].startswith("Basic ")

    @pytest.mark.asyncio
    async def test_apply_auth_async(self):
        """Test applying basic authentication asynchronously."""
        auth = BasicAuth("username", "password")
        request = httpx.Request("GET", "http://example.com")

        authenticated_request = await auth.apply_auth_async(request)

        assert "Authorization" in authenticated_request.headers
        assert authenticated_request.headers["Authorization"].startswith("Basic ")


class TestOAuth2ClientCredentialsAuth:
    """Test OAuth2ClientCredentialsAuth handler."""

    def test_initialization(self):
        """Test initialization of OAuth2 client credentials auth."""
        auth = OAuth2ClientCredentialsAuth(
            "https://example.com/token", "client_id", "client_secret", "read write"
        )

        assert auth.token_url == "https://example.com/token"
        assert auth.client_id == "client_id"
        assert auth.client_secret == "client_secret"
        assert auth.scope == "read write"
        assert auth._token is None

    def test_prepare_token_request(self):
        """Test token request preparation."""
        auth = OAuth2ClientCredentialsAuth(
            "https://example.com/token", "client_id", "client_secret", "read write"
        )

        data = auth._prepare_token_request()

        expected = {
            "grant_type": "client_credentials",
            "client_id": "client_id",
            "client_secret": "client_secret",
            "scope": "read write",
        }
        assert data == expected

    def test_prepare_token_request_no_scope(self):
        """Test token request preparation without scope."""
        auth = OAuth2ClientCredentialsAuth(
            "https://example.com/token", "client_id", "client_secret"
        )

        data = auth._prepare_token_request()

        expected = {
            "grant_type": "client_credentials",
            "client_id": "client_id",
            "client_secret": "client_secret",
        }
        assert data == expected

    def test_get_valid_token_sync(self):
        """Test getting valid token synchronously."""
        mock_client = Mock(spec=httpx.Client)
        mock_response = Mock()
        mock_response.raise_for_status.return_value = None
        mock_response.json.return_value = {
            "access_token": "new_token",
            "token_type": "Bearer",
            "expires_in": 3600,
        }
        mock_client.post.return_value = mock_response

        auth = OAuth2ClientCredentialsAuth(
            "https://example.com/token",
            "client_id",
            "client_secret",
            client=mock_client,
        )

        token = auth._get_valid_token_sync()

        assert token.access_token == "new_token"
        assert token.token_type == "Bearer"
        assert token.expires_in == 3600
        assert auth._token is token

        mock_client.post.assert_called_once_with(
            "https://example.com/token",
            data={
                "grant_type": "client_credentials",
                "client_id": "client_id",
                "client_secret": "client_secret",
            },
            headers={"Content-Type": "application/x-www-form-urlencoded"},
        )

    @pytest.mark.asyncio
    async def test_get_valid_token_async(self):
        """Test getting valid token asynchronously."""
        mock_client = AsyncMock(spec=httpx.AsyncClient)
        mock_response = Mock()
        mock_response.raise_for_status.return_value = None
        mock_response.json.return_value = {
            "access_token": "new_token",
            "token_type": "Bearer",
            "expires_in": 3600,
        }
        mock_client.post.return_value = mock_response

        auth = OAuth2ClientCredentialsAuth(
            "https://example.com/token",
            "client_id",
            "client_secret",
            client=mock_client,
        )

        token = await auth._get_valid_token_async()

        assert token.access_token == "new_token"
        assert token.token_type == "Bearer"
        assert token.expires_in == 3600
        assert auth._token is token

        mock_client.post.assert_called_once_with(
            "https://example.com/token",
            data={
                "grant_type": "client_credentials",
                "client_id": "client_id",
                "client_secret": "client_secret",
            },
            headers={"Content-Type": "application/x-www-form-urlencoded"},
        )

    def test_get_valid_token_sync_cached(self):
        """Test getting cached valid token synchronously."""
        mock_client = Mock(spec=httpx.Client)

        auth = OAuth2ClientCredentialsAuth(
            "https://example.com/token",
            "client_id",
            "client_secret",
            client=mock_client,
        )

        # Set a non-expired token
        auth._token = AuthToken("cached_token", expires_in=3600)

        token = auth._get_valid_token_sync()

        assert token.access_token == "cached_token"
        # Should not make HTTP request for valid cached token
        mock_client.post.assert_not_called()

    def test_apply_auth(self):
        """Test applying OAuth2 authentication synchronously."""
        mock_client = Mock(spec=httpx.Client)
        mock_response = Mock()
        mock_response.raise_for_status.return_value = None
        mock_response.json.return_value = {
            "access_token": "oauth_token",
            "token_type": "Bearer",
        }
        mock_client.post.return_value = mock_response

        auth = OAuth2ClientCredentialsAuth(
            "https://example.com/token",
            "client_id",
            "client_secret",
            client=mock_client,
        )

        request = httpx.Request("GET", "http://example.com")
        authenticated_request = auth.apply_auth(request)

        assert authenticated_request.headers["Authorization"] == "Bearer oauth_token"

    @pytest.mark.asyncio
    async def test_apply_auth_async(self):
        """Test applying OAuth2 authentication asynchronously."""
        mock_client = AsyncMock(spec=httpx.AsyncClient)
        mock_response = Mock()
        mock_response.raise_for_status.return_value = None
        mock_response.json.return_value = {
            "access_token": "oauth_token",
            "token_type": "Bearer",
        }
        mock_client.post.return_value = mock_response

        auth = OAuth2ClientCredentialsAuth(
            "https://example.com/token",
            "client_id",
            "client_secret",
            client=mock_client,
        )

        request = httpx.Request("GET", "http://example.com")
        authenticated_request = await auth.apply_auth_async(request)

        assert authenticated_request.headers["Authorization"] == "Bearer oauth_token"


class TestOAuth2AuthorizationCodeAuth:
    """Test OAuth2AuthorizationCodeAuth handler."""

    def test_initialization_minimal(self):
        """Test initialization with minimal parameters."""
        auth = OAuth2AuthorizationCodeAuth("access_token")

        assert auth._token.access_token == "access_token"
        assert auth._token.refresh_token is None
        assert auth.token_url is None

    def test_initialization_with_refresh(self):
        """Test initialization with refresh token."""
        auth = OAuth2AuthorizationCodeAuth(
            access_token="access_token",
            refresh_token="refresh_token",
            token_url="https://example.com/token",
            client_id="client_id",
            client_secret="client_secret",
            expires_in=3600,
        )

        assert auth._token.access_token == "access_token"
        assert auth._token.refresh_token == "refresh_token"
        assert auth._token.expires_in == 3600
        assert auth.token_url == "https://example.com/token"

    def test_initialization_invalid_refresh_config(self):
        """Test initialization with invalid refresh configuration."""
        with pytest.raises(
            ValueError, match="token_url, client_id, and client_secret are required"
        ):
            OAuth2AuthorizationCodeAuth(
                access_token="access_token",
                refresh_token="refresh_token",  # Has refresh token but missing other params
            )

    def test_refresh_token_sync(self):
        """Test token refresh synchronously."""
        mock_client = Mock(spec=httpx.Client)
        mock_response = Mock()
        mock_response.raise_for_status.return_value = None
        mock_response.json.return_value = {
            "access_token": "new_access_token",
            "token_type": "Bearer",
            "expires_in": 3600,
            "refresh_token": "new_refresh_token",
        }
        mock_client.post.return_value = mock_response

        auth = OAuth2AuthorizationCodeAuth(
            access_token="old_token",
            refresh_token="refresh_token",
            token_url="https://example.com/token",
            client_id="client_id",
            client_secret="client_secret",
            client=mock_client,
        )

        token = auth._refresh_token_sync()

        assert token.access_token == "new_access_token"
        assert token.refresh_token == "new_refresh_token"
        assert token.expires_in == 3600

        mock_client.post.assert_called_once_with(
            "https://example.com/token",
            data={
                "grant_type": "refresh_token",
                "refresh_token": "refresh_token",
                "client_id": "client_id",
                "client_secret": "client_secret",
            },
            headers={"Content-Type": "application/x-www-form-urlencoded"},
        )

    def test_refresh_token_sync_no_refresh_token(self):
        """Test token refresh without refresh token."""
        auth = OAuth2AuthorizationCodeAuth("access_token")

        with pytest.raises(RuntimeError, match="No refresh token available"):
            auth._refresh_token_sync()

    @pytest.mark.asyncio
    async def test_refresh_token_async(self):
        """Test token refresh asynchronously."""
        mock_client = AsyncMock(spec=httpx.AsyncClient)
        mock_response = Mock()
        mock_response.raise_for_status.return_value = None
        mock_response.json.return_value = {
            "access_token": "new_access_token",
            "token_type": "Bearer",
            "expires_in": 3600,
        }
        mock_client.post.return_value = mock_response

        auth = OAuth2AuthorizationCodeAuth(
            access_token="old_token",
            refresh_token="refresh_token",
            token_url="https://example.com/token",
            client_id="client_id",
            client_secret="client_secret",
            client=mock_client,
        )

        token = await auth._refresh_token_async()

        assert token.access_token == "new_access_token"
        assert token.expires_in == 3600

        mock_client.post.assert_called_once()

    def test_get_valid_token_sync_not_expired(self):
        """Test getting valid token when current token is not expired."""
        auth = OAuth2AuthorizationCodeAuth("access_token", expires_in=3600)

        token = auth._get_valid_token_sync()

        assert token.access_token == "access_token"

    def test_get_valid_token_sync_expired_no_refresh(self):
        """Test getting valid token when expired and no refresh token."""
        with patch("time.time", return_value=1000):
            auth = OAuth2AuthorizationCodeAuth("access_token", expires_in=60)

        with patch("time.time", return_value=1200):  # Expired
            with pytest.raises(
                RuntimeError,
                match="Access token is expired and no refresh token available",
            ):
                auth._get_valid_token_sync()

    def test_apply_auth(self):
        """Test applying OAuth2 authorization code authentication."""
        auth = OAuth2AuthorizationCodeAuth("access_token", expires_in=3600)
        request = httpx.Request("GET", "http://example.com")

        authenticated_request = auth.apply_auth(request)

        assert authenticated_request.headers["Authorization"] == "Bearer access_token"

    @pytest.mark.asyncio
    async def test_apply_auth_async(self):
        """Test applying OAuth2 authorization code authentication asynchronously."""
        auth = OAuth2AuthorizationCodeAuth("access_token", expires_in=3600)
        request = httpx.Request("GET", "http://example.com")

        authenticated_request = await auth.apply_auth_async(request)

        assert authenticated_request.headers["Authorization"] == "Bearer access_token"


class TestCustomAuth:
    """Test CustomAuth handler."""

    def test_initialization_sync_only(self):
        """Test initialization with sync function only."""

        def sync_auth(request):
            request.headers["Custom-Auth"] = "sync"
            return request

        auth = CustomAuth(sync_auth_func=sync_auth)
        assert auth.sync_auth_func is sync_auth
        assert auth.async_auth_func is None

    def test_initialization_async_only(self):
        """Test initialization with async function only."""

        async def async_auth(request):
            request.headers["Custom-Auth"] = "async"
            return request

        auth = CustomAuth(async_auth_func=async_auth)
        assert auth.sync_auth_func is None
        assert auth.async_auth_func is async_auth

    def test_initialization_both(self):
        """Test initialization with both functions."""

        def sync_auth(request):
            return request

        async def async_auth(request):
            return request

        auth = CustomAuth(sync_auth_func=sync_auth, async_auth_func=async_auth)
        assert auth.sync_auth_func is sync_auth
        assert auth.async_auth_func is async_auth

    def test_initialization_neither(self):
        """Test initialization with no functions raises error."""
        with pytest.raises(
            ValueError,
            match="At least one of sync_auth_func or async_auth_func must be provided",
        ):
            CustomAuth()

    def test_apply_auth(self):
        """Test applying custom synchronous authentication."""

        def sync_auth(request):
            request.headers["Custom-Auth"] = "test"
            return request

        auth = CustomAuth(sync_auth_func=sync_auth)
        request = httpx.Request("GET", "http://example.com")

        authenticated_request = auth.apply_auth(request)

        assert authenticated_request.headers["Custom-Auth"] == "test"

    def test_apply_auth_no_sync_func(self):
        """Test applying sync auth without sync function raises error."""

        async def async_auth(request):
            return request

        auth = CustomAuth(async_auth_func=async_auth)
        request = httpx.Request("GET", "http://example.com")

        with pytest.raises(RuntimeError, match="No synchronous auth function provided"):
            auth.apply_auth(request)

    @pytest.mark.asyncio
    async def test_apply_auth_async(self):
        """Test applying custom asynchronous authentication."""

        async def async_auth(request):
            request.headers["Custom-Auth"] = "test"
            return request

        auth = CustomAuth(async_auth_func=async_auth)
        request = httpx.Request("GET", "http://example.com")

        authenticated_request = await auth.apply_auth_async(request)

        assert authenticated_request.headers["Custom-Auth"] == "test"

    @pytest.mark.asyncio
    async def test_apply_auth_async_no_async_func(self):
        """Test applying async auth without async function raises error."""

        def sync_auth(request):
            return request

        auth = CustomAuth(sync_auth_func=sync_auth)
        request = httpx.Request("GET", "http://example.com")

        with pytest.raises(
            RuntimeError, match="No asynchronous auth function provided"
        ):
            await auth.apply_auth_async(request)


class TestConvenienceFunctions:
    """Test convenience factory functions."""

    def test_bearer_token_factory(self):
        """Test bearer_token factory function."""
        auth = bearer_token("test_token")
        assert isinstance(auth, BearerTokenAuth)
        assert auth.token == "test_token"

    def test_api_key_factory(self):
        """Test api_key factory function."""
        auth = api_key("test_key")
        assert isinstance(auth, APIKeyAuth)
        assert auth.api_key == "test_key"
        assert auth.header_name == "X-API-Key"

    def test_api_key_factory_with_params(self):
        """Test api_key factory function with parameters."""
        auth = api_key("test_key", header_name="Authorization", param_name="key")
        assert isinstance(auth, APIKeyAuth)
        assert auth.api_key == "test_key"
        assert auth.header_name == "Authorization"
        assert auth.param_name == "key"

    def test_basic_auth_factory(self):
        """Test basic_auth factory function."""
        auth = basic_auth("user", "pass")
        assert isinstance(auth, BasicAuth)

    def test_oauth2_client_credentials_factory(self):
        """Test oauth2_client_credentials factory function."""
        auth = oauth2_client_credentials(
            "https://example.com/token", "client_id", "client_secret", "read write"
        )
        assert isinstance(auth, OAuth2ClientCredentialsAuth)
        assert auth.token_url == "https://example.com/token"
        assert auth.client_id == "client_id"
        assert auth.client_secret == "client_secret"
        assert auth.scope == "read write"

    def test_oauth2_authorization_code_factory(self):
        """Test oauth2_authorization_code factory function."""
        auth = oauth2_authorization_code(
            "access_token",
            refresh_token="refresh_token",
            token_url="https://example.com/token",
            client_id="client_id",
            client_secret="client_secret",
            expires_in=3600,
        )
        assert isinstance(auth, OAuth2AuthorizationCodeAuth)
        assert auth._token.access_token == "access_token"
        assert auth._token.refresh_token == "refresh_token"
        assert auth.token_url == "https://example.com/token"


class TestClientIntegration:
    """Test integration of authentication with client classes."""

    def test_sync_client_with_bearer_token(self):
        """Test sync client with bearer token authentication."""
        mock_client = Mock(spec=httpx.Client)
        mock_response = Mock(spec=httpx.Response)
        mock_response.status_code = 200
        mock_response.headers = httpx.Headers({})
        mock_response.reason_phrase = "OK"
        mock_response.json.return_value = {"success": True}
        mock_response.content = b'{"success": true}'

        mock_request = Mock(spec=httpx.Request)
        mock_client.build_request.return_value = mock_request
        mock_client.send.return_value = mock_response

        client = ClientBase.from_bearer_token(
            "http://example.com", "test_token", client=mock_client
        )

        # Verify the auth handler was set
        assert isinstance(client.auth, BearerTokenAuth)
        assert client.auth.token == "test_token"

    def test_async_client_with_api_key(self):
        """Test async client with API key authentication."""
        mock_client = AsyncMock(spec=httpx.AsyncClient)
        mock_response = Mock(spec=httpx.Response)
        mock_response.status_code = 200
        mock_response.headers = httpx.Headers({})
        mock_response.reason_phrase = "OK"
        mock_response.json.return_value = {"success": True}
        mock_response.content = b'{"success": true}'

        mock_request = Mock(spec=httpx.Request)
        mock_client.build_request.return_value = mock_request
        mock_client.send = AsyncMock(return_value=mock_response)

        client = AsyncClientBase.from_api_key(
            "http://example.com", "test_key", client=mock_client
        )

        # Verify the auth handler was set
        assert isinstance(client.auth, APIKeyAuth)
        assert client.auth.api_key == "test_key"

    def test_sync_client_with_oauth2_client_credentials(self):
        """Test sync client with OAuth2 client credentials."""
        mock_client = Mock(spec=httpx.Client)

        client = ClientBase.from_oauth2_client_credentials(
            "http://example.com",
            "https://auth.example.com/token",
            "client_id",
            "client_secret",
            "read write",
            client=mock_client,
        )

        assert isinstance(client.auth, OAuth2ClientCredentialsAuth)
        assert client.auth.token_url == "https://auth.example.com/token"
        assert client.auth.client_id == "client_id"
        assert client.auth.client_secret == "client_secret"
        assert client.auth.scope == "read write"

    def test_sync_client_with_basic_auth(self):
        """Test sync client with basic authentication."""
        mock_client = Mock(spec=httpx.Client)

        client = ClientBase.from_basic_auth(
            "http://example.com", "username", "password", client=mock_client
        )

        assert isinstance(client.auth, BasicAuth)
        assert isinstance(client.auth.auth, httpx.BasicAuth)
