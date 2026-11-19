# Introduction

`reflectapi`: Define your web API in Rust, generate typed TypeScript, Python and Rust clients. Serve it with Axum, Cloudflare Workers or any HTTP server.

## Why `reflectapi`?

`reflectapi` was built at Partly, where Rust services are called by
TypeScript web apps and by other services. With new endpoints and type
changes every day, hand-written clients kept drifting from the server.
Generating them through OpenAPI did not fix it: across several generators,
the clients were too often incompatible with what the server actually sent,
and the workarounds (patching generated code, mixing generators, going back
to hand-written clients for some routes) cost more than they saved.

The problem is the route through OpenAPI, not any one generator:

- **Type information is lost.** Generics, enums with data, structured errors
  and module namespaces have no exact OpenAPI equivalent, so they are
  flattened or renamed on the way out.
- **Generators must handle all of HTTP**, though an API call needs only a
  name, an input type and an output type, its errors included.
- **OpenAPI types are constraints, not definitions.** A generator has to
  guess which type a set of validators describes, and that guess cannot be
  made reliably in general.

`reflectapi` skips that step. Its derive macros read the Rust types and their
serde attributes into its own schema with nothing lost, and the clients are
generated from that schema. A generated client function has the same name,
input and output as the Rust handler it calls, and a change on the server
shows up as a compile error in a TypeScript or Rust client. An OpenAPI
document can still be written from the schema, for documentation and tools
that need one.

The full story, with a live demo, is in the talk
[Reflect your Rust API into the world](https://www.youtube.com/watch?v=6Scgq9fBZQM&t=21675s)
at Rust Forge 2025 in Wellington, New Zealand.

## Core Philosophy

1. Rust code first definition of the API interface
2. Full respect for all serde attributes
3. Extensible at many places

## Where to go next

- **New to reflectapi:** the [Quick Start](./getting-started/quick-start.md)
  builds an API and calls it from TypeScript in a few minutes. Then read
  [How It Works](./concepts/how-it-works.md).
- **Choosing a tool:** [alternatives](./guides/alternatives.md), such as
  utoipa, aide and OpenAPI generators, and when each fits.
- **Building a real API:** the Concepts section covers errors, types,
  organising and evolving an API, and testing. The Reference section lists
  every attribute and how each client language behaves.
- **API reference:** every type and function is on
  [docs.rs](https://docs.rs/reflectapi).
