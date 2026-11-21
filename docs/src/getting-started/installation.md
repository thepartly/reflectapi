# Installation

Get `reflectapi` up and running in minutes.

## Basic Setup

```shell
cargo add reflectapi --features builder,axum,json,chrono
```

## CLI Tool

Install the CLI tool to generate client libraries:

```bash
cargo install reflectapi-cli
```

This installs the `reflectapi` binary.

### Keep the CLI at least as new as the server

The server writes its schema (`reflectapi.json`) with the version of the `reflectapi` crate it's built with, and the CLI reads it. These are often different releases, so the schema records the format it's written in as `format_version`.

- **A CLI reads any schema format up to its own.** A newer CLI reads an older schema.
- **A CLI refuses a newer format** and says to upgrade it, rather than generating a client that's silently wrong.
- **Within a format, changes are additive.** An older CLI that ignores something new, such as a deprecation or a `distinct` type, still generates a client that sends and accepts the same JSON. It just has less type information. Anything an older CLI would get wrong by ignoring it, such as a new kind of route, moves to a new format.

So when you upgrade `reflectapi` on the server, upgrade `reflectapi-cli` wherever clients are generated from its schema.

## Next Steps

- **New users**: Follow the [Quick Start](./quick-start.md) guide
