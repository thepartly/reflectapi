# Alternatives: utoipa, aide, OpenAPI Generators and Type Sharing

Other ways to connect a Rust server to its clients, and when each fits.

- **[utoipa](https://github.com/juhaku/utoipa)**: you keep your web
  framework's handlers (axum, actix-web, Rocket and others) and want an
  OpenAPI document generated from annotations on them.
- **[aide](https://github.com/tamasfe/aide)**: you build with axum and want
  the OpenAPI document derived from your routes and extractors.
- **OpenAPI client generators**, such as
  [openapi-generator](https://openapi-generator.tech),
  [openapi-typescript](https://openapi-ts.dev),
  [orval](https://orval.dev) and
  [progenitor](https://github.com/oxidecomputer/progenitor): an OpenAPI
  document is the contract, or you need clients in languages reflectapi
  doesn't generate. reflectapi can write that document too; see
  [Supported Outputs](../clients/index.md#supported-outputs).
- **[ts-rs](https://github.com/Aleph-Alpha/ts-rs),
  [specta](https://github.com/specta-rs/specta) and
  [typeshare](https://github.com/1Password/typeshare)**: you only need your
  Rust types in TypeScript or other languages, and write the calls yourself.
- **[tonic](https://github.com/hyperium/tonic)** with Protocol Buffers: the
  contract is a `.proto` file, and clients and servers speak gRPC.

reflectapi fits when you own the server and its clients, routes can be
named calls rather than REST resources, and the Rust types and their serde
attributes should be the only definition of the API.
[Why reflectapi?](../introduction.md#why-reflectapi) explains why it doesn't
build on OpenAPI.
