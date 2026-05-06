# reflectapi-schema-codegen

Compiler-owned schema normalization and semantic IR for ReflectAPI.

This crate depends on `reflectapi-schema` and owns:

- compiler-side symbol identity
- normalization passes over raw schema
- semantic IR construction for codegen backends

It is intentionally separate from the raw schema crate so the serialized schema
format stays focused on interchange concerns rather than compiler metadata.
