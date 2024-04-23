# ReflectAPI

ReflectAPI is a library for Rust code-first web service API declaration and corresponding clients code generation tools.

More documentation will follow later.


### Development notes

To run the demo server:
```
cargo run --bin reflectapi-demo
```

To generate client in Typescript for demo server:
```
cargo run --bin reflectapi-cli -- codegen --language typescript --schema reflectapi-demo/reflectapi.json --output reflectapi-demo/clients/typescript
```

To run the Typescript generated client. Note: requires the demo server running
```
cd reflectapi-demo/clients/typescript/
pnpm install
pnpm run start
```
