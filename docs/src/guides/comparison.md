# reflectapi vs utoipa, aide and OpenAPI Client Generators

There are several ways to get typed clients for a Rust web API. Most go
through OpenAPI: describe the API with a Rust crate such as utoipa or aide,
export an OpenAPI document, then run a client generator on it. `reflectapi`
generates its clients from its own schema instead, and produces OpenAPI as one
more output. This page compares the two approaches, and the type-only tools,
so you can pick the one that fits.

## The OpenAPI pipeline

1. **Describe the API in the server.** utoipa uses derive macros on types and
   attribute macros on handlers, and works with several web frameworks. aide
   builds the document from axum routes and extractors, using schemars for the
   types. In both, handlers stay ordinary framework handlers.
2. **Export the OpenAPI document**, at build time or from an endpoint.
3. **Generate clients** with a separate tool: openapi-generator (many
   languages), openapi-typescript (types, with openapi-fetch for calls) or
   orval for TypeScript, progenitor for Rust, and others.

This is the right choice when OpenAPI is the contract: a public REST API,
other teams generating their own clients, or tooling such as documentation
portals, gateways and contract tests that read OpenAPI. It describes REST
URLs with path parameters, query strings and any HTTP method, and it reaches
every language the generators support.

The cost is two translations, Rust to OpenAPI and OpenAPI to the client
language, each done by a different tool with its own type mapping. How serde
features such as tagged enums, `flatten` and generics come out depends on the
tools chosen, and it is worth checking that the generated client reads what
the server actually sends.

## The reflectapi pipeline

1. **Define the API with `reflectapi`.** Types derive `Input` or `Output`
   next to serde's `Deserialize` or `Serialize`. Handlers are async functions
   taking state, input and headers, registered on a `reflectapi::Builder`.
2. **Build.** `build()` returns the schema, which you write to
   `reflectapi.json`, and the routes.
3. **Generate clients** from the schema with `reflectapi codegen`: TypeScript,
   Rust and Python. `--language openapi` writes an OpenAPI 3.1 document from
   the same schema.
4. **Serve** the routes with [axum](../serving/axum.md),
   [Cloudflare Workers or celld](../serving/workers.md), or
   [any other server](../serving/other-servers.md).

One project owns the whole path from Rust types to client code, so the
clients follow the server's serde attributes. These also carry into the
generated clients: separate input and output shapes for the same type, typed
errors, declared response headers, Server-Sent Event streams, deprecations
and [open enums](../getting-started/attributes.md#enum-evolution).

The constraints:

- **RPC-style routes.** Each route is a named endpoint at a fixed path,
  `POST {path}/{name}`, plus `GET` for read-only routes. Input is the request
  body and a struct of headers. There are no path parameters or query strings,
  so `reflectapi` cannot describe an arbitrary existing REST API.
- **Its own handler signature.** Handlers are not axum handlers and do not use
  axum extractors. Middleware is added as tower layers when
  [mounting the routes](../serving/axum.md#mount-the-routes).
- **Three client languages.** TypeScript and Rust are stable, Python is
  experimental. For other languages, feed the OpenAPI output to an OpenAPI
  generator. That output is an approximation of the schema: for example,
  generic types are expanded per use, and the unknown case that generated
  clients add for open enums is not in it.

## Type-only tools

ts-rs, specta and typeshare generate type definitions from Rust types:
TypeScript, and for typeshare several other languages. They do not describe
endpoints, so you write the HTTP calls yourself. They fit when you only need
shared types, or when the transport is not an HTTP API at all.

## Which to choose

| Situation | Better fit |
|-----------|------------|
| Existing axum or other framework handlers you want to keep and document | utoipa or aide |
| REST URLs with path parameters and query strings | utoipa or aide, or a spec-first workflow |
| OpenAPI document is the source of truth | OpenAPI client generators |
| Clients in languages other than TypeScript, Rust and Python | OpenAPI generators (possibly fed by `reflectapi`'s OpenAPI output) |
| Only shared types, no generated calls | ts-rs, specta or typeshare |
| You own the server and its clients, RPC-style routes are fine, and clients should follow the Rust types and serde attributes directly | `reflectapi` |

To try `reflectapi`, start with the [Quick Start](../getting-started/quick-start.md),
or the guides for a [TypeScript client](./typescript-client-axum.md) and a
[Python client](./python-client.md).
