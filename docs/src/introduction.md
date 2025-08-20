# Introduction

ReflectAPI is a Rust library for code-first web service API declaration and client generation. It allows you to define your API using native Rust types and automatically generate type-safe client libraries for TypeScript, Python, and Rust.

## Why ReflectAPI?

Traditional API development often involves:
- ❌ Maintaining separate API specifications (OpenAPI, etc.)
- ❌ Keeping client libraries in sync with server changes
- ❌ Manual type definitions across multiple languages
- ❌ Runtime errors from API mismatches

ReflectAPI solves these problems by:
- ✅ **Code-first approach**: API definition lives in your Rust code
- ✅ **Automatic client generation**: Generate clients in multiple languages
- ✅ **Type safety**: Compile-time guarantees across all clients
- ✅ **Single source of truth**: Your Rust types define everything

## Core Philosophy

ReflectAPI follows a simple principle: **define once, generate everything**. By leveraging Rust's powerful type system and derive macros, you can:

1. Define your API types using standard Rust structs and enums
2. Add derive macros to make them API-compatible
3. Use the builder pattern to define endpoints
4. Generate clients for any supported language

## What You'll Learn

This documentation will guide you through:
- Setting up ReflectAPI in your project
- Defining type-safe APIs using Rust
- Generating clients for different languages
- Advanced features like custom types and validation
- Best practices for production APIs

## Ready to Start?

Head over to [Quick Start](./getting-started/quick-start.md) to build your first API with ReflectAPI!