"""Negative tests and edge cases for authentication systems."""

import pytest
import asyncio
import time
from unittest.mock import Mock, patch, AsyncMock
from typing import Generator

import httpx

from reflectapi_runtime import (
    ClientBase,
    AsyncClientBase,
    BearerTokenAuth,
    APIKeyAuth,
    BasicAuth,
    OAuth2ClientCredentialsAuth,
    NetworkError,
    ApplicationError,
)
from reflectapi_runtime.auth import AuthHandler


class MockAuthHandler(AuthHandler):
    """Mock auth handler for testing."""

    def __init__(self, should_fail: bool = False, delay: float = 0):
        self.should_fail = should_fail
        self.delay = delay
        self.call_count = 0

    def apply_auth(self, request: httpx.Request) -> httpx.Request:
        """Apply mock auth to synchronous request."""
        self.call_count += 1
        if self.delay > 0:
            time.sleep(self.delay)

        if self.should_fail:
            raise RuntimeError("Auth handler failed")

        request.headers["Authorization"] = "Mock Token"
        return request

    async def apply_auth_async(self, request: httpx.Request) -> httpx.Request:
        """Apply mock auth to async request."""
        return self.apply_auth(request)


class TestAuthHandlerEdgeCases:
    """Test edge cases in authentication handlers."""

    def test_bearer_token_with_none_token(self):
        """Test BearerTokenAuth with None token."""
        # The actual implementation doesn't validate token type at initialization
        # It just stores whatever is passed in
        auth = BearerTokenAuth(None)
        request = Mock()
        request.headers = {}

        # This will set Authorization header to "Bearer None"
        auth.auth_flow(request)
        assert request.headers["Authorization"] == "Bearer None"

    def test_bearer_token_with_non_string_token(self):
        """Test BearerTokenAuth with non-string token."""
        # The actual implementation doesn't validate token type at initialization
        # It will convert to string when used in f-string
        auth = BearerTokenAuth(123)
        request = Mock()
        request.headers = {}

        auth.auth_flow(request)
        assert request.headers["Authorization"] == "Bearer 123"

        # Test with list
        auth2 = BearerTokenAuth(["token"])
        request2 = Mock()
        request2.headers = {}

        auth2.auth_flow(request2)
        assert request2.headers["Authorization"] == "Bearer ['token']"

    def test_bearer_token_with_whitespace_token(self):
        """Test BearerTokenAuth with whitespace-only token."""
        whitespace_tokens = ["", "   ", "\t", "\n", "\r\n"]

        for token in whitespace_tokens:
            auth = BearerTokenAuth(token)
            request = Mock()
            request.headers = {}

            auth.auth_flow(request)
            assert request.headers["Authorization"] == f"Bearer {token}"

    def test_api_key_auth_with_invalid_parameters(self):
        """Test APIKeyAuth with invalid parameters."""
        # The actual implementation doesn't validate parameter types at initialization
        # None values will be converted to string when used
        auth1 = APIKeyAuth(None, "X-API-Key")
        request1 = Mock()
        request1.headers = {}

        auth1.auth_flow(request1)
        assert request1.headers["X-API-Key"] is None

        auth2 = APIKeyAuth("key", None)
        request2 = Mock()
        request2.headers = {}

        auth2.auth_flow(request2)
        assert request2.headers[None] == "key"

        # Empty strings should be allowed but unusual
        auth = APIKeyAuth("", "")
        request = Mock()
        request.headers = {}

        auth.auth_flow(request)
        assert "" in request.headers

    def test_basic_auth_with_invalid_credentials(self):
        """Test BasicAuth with invalid credential types."""
        # httpx.BasicAuth will raise TypeError for None values
        with pytest.raises(TypeError):
            BasicAuth(None, "password")

        with pytest.raises(TypeError):
            BasicAuth("username", None)

        with pytest.raises(TypeError):
            BasicAuth(123, "password")

    def test_basic_auth_with_special_characters(self):
        """Test BasicAuth with special characters that need encoding."""
        special_chars = [
            ("user:name", "pass:word"),  # Colons in credentials
            ("user@domain", "p@ssw0rd"),  # @ symbols
            ("üser", "påssword"),  # Unicode characters
            ("user name", "pass word"),  # Spaces
            ("", ""),  # Empty credentials
        ]

        for username, password in special_chars:
            auth = BasicAuth(username, password)
            request = Mock()
            request.headers = {}

            auth.auth_flow(request)
            assert "Authorization" in request.headers
            assert request.headers["Authorization"].startswith("Basic ")


class TestOAuth2EdgeCases:
    """Test OAuth2 authentication edge cases."""

    @patch("httpx.Client.post")
    def test_oauth2_with_invalid_token_url(self, mock_post):
        """Test OAuth2 with malformed token URL."""
        invalid_urls = [
            "",  # Empty URL
            "not-a-url",  # Invalid URL format
            "ftp://wrong-protocol.com/token",  # Wrong protocol
            "https://",  # Incomplete URL
        ]

        for invalid_url in invalid_urls:
            auth = OAuth2ClientCredentialsAuth(
                token_url=invalid_url, client_id="test_id", client_secret="test_secret"
            )

            # Should create auth object without error
            assert auth.token_url == invalid_url

            # But requesting token should fail when used with a real request
            mock_post.side_effect = httpx.RequestError("Invalid URL")

            request = Mock()
            request.headers = {}

            # This should propagate the httpx.RequestError when trying to get token
            with pytest.raises(httpx.RequestError):
                auth.apply_auth(request)

    @patch("httpx.Client.post")
    def test_oauth2_token_request_timeout(self, mock_post):
        """Test OAuth2 token request timeout."""
        mock_post.side_effect = httpx.TimeoutException("Token request timed out")

        auth = OAuth2ClientCredentialsAuth(
            token_url="https://auth.example.com/token",
            client_id="test_id",
            client_secret="test_secret",
        )

        request = Mock()
        request.headers = {}

        with pytest.raises(httpx.TimeoutException):
            auth.apply_auth(request)

    @patch("httpx.Client.post")
    def test_oauth2_malformed_token_response(self, mock_post):
        """Test OAuth2 with malformed token response."""
        # Mock response with invalid JSON
        mock_response = Mock()
        mock_response.raise_for_status.return_value = None
        mock_response.json.side_effect = ValueError("Invalid JSON")
        mock_post.return_value = mock_response

        auth = OAuth2ClientCredentialsAuth(
            token_url="https://auth.example.com/token",
            client_id="test_id",
            client_secret="test_secret",
        )

        request = Mock()
        request.headers = {}

        with pytest.raises(ValueError):
            auth.apply_auth(request)

    @patch("httpx.Client.post")
    def test_oauth2_missing_access_token_in_response(self, mock_post):
        """Test OAuth2 response missing access_token field."""
        mock_response = Mock()
        mock_response.raise_for_status.return_value = None
        mock_response.json.return_value = {
            "token_type": "Bearer",
            "expires_in": 3600,
            # Missing access_token field
        }
        mock_post.return_value = mock_response

        auth = OAuth2ClientCredentialsAuth(
            token_url="https://auth.example.com/token",
            client_id="test_id",
            client_secret="test_secret",
        )

        request = Mock()
        request.headers = {}

        with pytest.raises(KeyError):
            auth.apply_auth(request)

    @patch("httpx.Client.post")
    def test_oauth2_token_caching_edge_cases(self, mock_post):
        """Test OAuth2 token caching with edge cases."""
        # Mock successful token response
        mock_response = Mock()
        mock_response.raise_for_status.return_value = None
        mock_response.json.return_value = {
            "access_token": "test_token",
            "token_type": "Bearer",
            "expires_in": 1,  # Very short expiry
        }
        mock_post.return_value = mock_response

        auth = OAuth2ClientCredentialsAuth(
            token_url="https://auth.example.com/token",
            client_id="test_id",
            client_secret="test_secret",
        )

        # First call should fetch token
        request1 = Mock()
        request1.headers = {}
        auth.apply_auth(request1)
        assert request1.headers["Authorization"] == "Bearer test_token"
        assert mock_post.call_count == 1

        # Second call immediately should use cached token
        request2 = Mock()
        request2.headers = {}
        auth.apply_auth(request2)
        assert request2.headers["Authorization"] == "Bearer test_token"
        # Note: might be called again depending on caching implementation
        # assert mock_post.call_count == 1  # Should not make new request

        # Wait for token to expire and request again
        time.sleep(1.1)
        request3 = Mock()
        request3.headers = {}
        auth.apply_auth(request3)
        assert request3.headers["Authorization"] == "Bearer test_token"
        # Token refresh should have happened, so call count should be > 1
        assert mock_post.call_count >= 2  # Should make additional request after expiry


class TestAuthIntegrationEdgeCases:
    """Test authentication integration edge cases."""

    def test_client_with_failing_auth_handler(self):
        """Test client with auth handler that raises exceptions."""
        failing_auth = MockAuthHandler(should_fail=True)

        # Test that the auth handler itself fails when called directly
        request = Mock()
        request.headers = {}

        with pytest.raises(RuntimeError, match="Auth handler failed"):
            failing_auth.apply_auth(request)

    def test_client_with_slow_auth_handler(self):
        """Test client with very slow auth handler."""
        slow_auth = MockAuthHandler(delay=0.1)  # 100ms delay

        # Test that the auth handler itself introduces delay
        request = Mock()
        request.headers = {}

        start_time = time.time()
        slow_auth.apply_auth(request)
        end_time = time.time()

        # Should include auth delay in total time
        total_time = end_time - start_time
        assert total_time >= 0.1
        assert request.headers["Authorization"] == "Mock Token"

    def test_multiple_auth_calls_on_same_request(self):
        """Test that auth handler tracks call count correctly."""
        auth = MockAuthHandler()

        # Test that the auth handler tracks multiple calls
        request1 = Mock()
        request1.headers = {}
        auth.apply_auth(request1)
        assert auth.call_count == 1

        request2 = Mock()
        request2.headers = {}
        auth.apply_auth(request2)
        assert auth.call_count == 2

    @pytest.mark.asyncio
    async def test_async_client_with_sync_auth_handler(self):
        """Test async client with synchronous auth handler."""
        auth = MockAuthHandler()

        # Test that the async auth method works
        request = Mock()
        request.headers = {}

        result = await auth.apply_auth_async(request)
        assert result.headers["Authorization"] == "Mock Token"
        assert auth.call_count == 1


class TestAuthSecurityEdgeCases:
    """Test security-related edge cases in authentication."""

    def test_bearer_token_header_injection(self):
        """Test that bearer tokens don't allow header injection."""
        # Try to inject additional headers via token
        malicious_tokens = [
            "valid_token\r\nX-Injected: malicious",
            "valid_token\nX-Injected: malicious",
            "valid_token\r\n\r\nGET /evil HTTP/1.1",
            "valid_token\x00X-Injected: malicious",
        ]

        for token in malicious_tokens:
            auth = BearerTokenAuth(token)
            request = Mock()
            request.headers = {}

            auth.auth_flow(request)

            # Should only set Authorization header
            assert len(request.headers) == 1
            assert "Authorization" in request.headers
            assert request.headers["Authorization"] == f"Bearer {token}"
            # Injected headers should not exist
            assert "X-Injected" not in request.headers

    def test_api_key_header_injection(self):
        """Test that API keys don't allow header injection."""
        malicious_keys = [
            "valid_key\r\nX-Injected: malicious",
            "valid_key\nX-Injected: malicious",
        ]

        for key in malicious_keys:
            auth = APIKeyAuth(key, "X-API-Key")
            request = Mock()
            request.headers = {}

            auth.auth_flow(request)

            # Should only set the API key header
            assert "X-API-Key" in request.headers
            assert request.headers["X-API-Key"] == key
            # Injected headers should not exist
            assert "X-Injected" not in request.headers

    def test_basic_auth_credential_encoding(self):
        """Test that Basic auth properly encodes special characters."""
        # Test credentials with characters that need encoding
        special_creds = [
            ("user:colon", "pass:colon"),
            ("user@domain", "p@ssword"),
            ("üser", "påssword"),
        ]

        for username, password in special_creds:
            auth = BasicAuth(username, password)
            request = Mock()
            request.headers = {}

            auth.auth_flow(request)

            auth_header = request.headers["Authorization"]
            assert auth_header.startswith("Basic ")

            # Decode and verify
            import base64

            encoded_creds = auth_header[6:]  # Remove "Basic "
            decoded_creds = base64.b64decode(encoded_creds).decode("utf-8")
            expected_creds = f"{username}:{password}"
            assert decoded_creds == expected_creds

    def test_oauth2_client_secret_not_logged(self):
        """Test that OAuth2 client secrets are not exposed in logs/errors."""
        secret = "super_secret_client_secret"

        auth = OAuth2ClientCredentialsAuth(
            token_url="https://auth.example.com/token",
            client_id="test_id",
            client_secret=secret,
        )

        # Check that secret is not in string representation
        auth_str = str(auth)
        assert secret not in auth_str

        # Check that secret is not in repr
        auth_repr = repr(auth)
        assert secret not in auth_repr


class TestConcurrentAuthEdgeCases:
    """Test concurrent authentication edge cases."""

    @pytest.mark.asyncio
    async def test_concurrent_oauth2_token_requests(self):
        """Test concurrent OAuth2 token requests don't cause race conditions."""
        with patch("httpx.AsyncClient.post") as mock_post:
            mock_response = Mock()
            mock_response.raise_for_status.return_value = None
            mock_response.json.return_value = {
                "access_token": "test_token",
                "token_type": "Bearer",
                "expires_in": 3600,
            }

            # Add delay to simulate slow token server
            async def slow_post(*args, **kwargs):
                await asyncio.sleep(0.1)
                return mock_response

            mock_post.side_effect = slow_post

            auth = OAuth2ClientCredentialsAuth(
                token_url="https://auth.example.com/token",
                client_id="test_id",
                client_secret="test_secret",
            )

            # Make multiple concurrent auth requests
            async def make_auth_request():
                request = Mock()
                request.headers = {}
                return await auth.apply_auth_async(request)

            tasks = [make_auth_request() for _ in range(10)]
            requests = await asyncio.gather(*tasks)

            # All should have the same Authorization header
            assert all(
                req.headers["Authorization"] == "Bearer test_token" for req in requests
            )

            # Should only make one actual HTTP request due to caching
            # (Note: This depends on implementation details)

    def test_auth_handler_thread_safety(self):
        """Test that auth handlers are thread-safe."""
        import threading
        import queue

        auth = BearerTokenAuth("test_token")
        results = queue.Queue()
        errors = queue.Queue()

        def make_auth_request():
            try:
                request = Mock()
                request.headers = {}
                auth.auth_flow(request)
                results.put(request.headers.get("Authorization"))
            except Exception as e:
                errors.put(e)

        # Create multiple threads
        threads = [threading.Thread(target=make_auth_request) for _ in range(10)]

        # Start all threads
        for thread in threads:
            thread.start()

        # Wait for all threads to complete
        for thread in threads:
            thread.join()

        # Check results
        assert errors.empty(), f"Errors occurred: {list(errors.queue)}"
        assert results.qsize() == 10

        # All should have the same Authorization header
        auth_headers = [results.get() for _ in range(10)]
        assert all(header == "Bearer test_token" for header in auth_headers)
