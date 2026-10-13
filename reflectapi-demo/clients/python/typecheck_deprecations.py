# pyright: reportDeprecated=error, reportUnnecessaryTypeIgnoreComment=error
"""Checked by pyright in CI, never run.

Each call below is to a deprecated method, so pyright must report it under
`reportDeprecated`. The ignore comment suppresses that report; if the
generated client stops marking the method deprecated, the ignore becomes
unnecessary and `reportUnnecessaryTypeIgnoreComment` fails the check.
"""

from api_client import AsyncClient, Client


async def call_deprecated_async(client: AsyncClient) -> None:
    await client.pets.delete()  # pyright: ignore[reportDeprecated]


def call_deprecated_sync(client: Client) -> None:
    client.pets.delete()  # pyright: ignore[reportDeprecated]
