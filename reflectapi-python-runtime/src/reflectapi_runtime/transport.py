"""Transport-shape DTOs and Protocols shared by client and middleware.

Lives in its own module so that ``middleware`` can depend on these types
without creating a circular import with ``client``.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Protocol, runtime_checkable


@dataclass(frozen=True)
class Request:
    """Transport request passed to custom ReflectAPI Python clients.

    ``method`` is intentionally absent: every reflectapi endpoint is POST
    by design, so transports hardcode it. If that ever changes it's a
    wire-protocol break and clients regenerate.
    """

    path: str
    headers: dict[str, str]
    body: bytes


@dataclass(frozen=True)
class Response:
    """Transport response returned by custom ReflectAPI Python clients."""

    status: int
    # Permissive headers type so adapters wrapping httpx.Headers (which is
    # case-insensitive) can hand it through verbatim instead of materialising
    # a plain dict on the hot path.
    headers: Any
    body: bytes


@runtime_checkable
class Client(Protocol):
    """Synchronous transport protocol for generated clients.

    Implementations return any object with ``.status`` / ``.headers`` /
    ``.body``; the provided :class:`Response` dataclass is a convenient
    ready-made implementation, not a hard requirement.
    """

    def request(self, request: Request) -> Response: ...


@runtime_checkable
class AsyncClient(Protocol):
    """Asynchronous transport protocol for generated clients.

    See :class:`Client` for the structural-response contract.
    """

    async def request(self, request: Request) -> Response: ...
