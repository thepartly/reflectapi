# API Design Standard

Conventions for designing consistent API schemas, and for implementing the
applications and servers that expose them over HTTP.

The standard is opinionated, and its opinion comes from one goal: a function
on the server reaches every client as the same function, with the same
signature, input, output and error types. An API call needs a name, an input
type and an output type, its errors included; that's all. So the standard
cuts HTTP down to what that needs, with one way to do each thing: the call's
name in the path, its input and output always objects in the body, `POST`
always working. Every rule below serves making an endpoint easy to define,
understand and implement, and easy to reflect into clients.

reflectapi doesn't require any of this: it is one style, and
[`reflectapi::conventions`](./conventions.md) has the types that implement
its pagination and conflict resolution. Take it as it is, or as a starting
point to adapt. [The Rust Forge 2025 talk](../talk.md) tells how it came
about.

## At a glance

<div class="glance-wrap">
{{#include glance.svg}}
</div>

## Request method

1. Every write and mutation is a `POST`.
2. Every read-only call supports both `POST` and `GET`, with or without
   parameters (see [Request body](#request-body)).
   - Only `POST` appears in the schema (see
     [`openapi-schema.get`](#openapi-schemaget)).
   - `GET` is supported implicitly: the server handles it as it handles `POST`
     for the same path, but it isn't in the schema.

<details class="reasoning">
<summary>Reasoning</summary>

- **The method is in the path, not in the HTTP verb:**
  - HTTP verbs are never enough to cover every operation. An API that starts
    with them ends up as a hybrid, with some verbs in the path anyway, so we
    follow Slack's convention of putting all of them in the path.
  - A call name in the path is easy to find in code (see
    [Implementation](#implementation)).
  - The call name is the call's complete signature, which also makes it
    visible in tracing tools.
  - Nobody has to remember which method to use: `POST` always works.
- **Only `POST` is in the schema for read-only calls:**
  - `GET` is a convenience, intuitive and easier with tools like `curl`, but
    adds nothing `POST` can't do.
  - Listing both would duplicate every read-only call in the schema.

</details>

## URL

```text
http[s]://<endpoint-address>/api/<version>/<call-name>
```

- A `call-name` is `resources[.nested-resources[.sub-nested-resources]].action-verb`:
  - `resources` is always required, except when `action-verb` is `lookup`.
  - `action-verb` is always required, even for `get` without parameters.
  - Parts are separated by `.`.
  - A resource or action of several words is `kebab-case`.

  Examples: `users.get`, `products.integrations.list`,
  `product-groups.attributes.insert`.

- `version` is the API's major version. A new major version is introduced
  only for a change that breaks compatibility, in the API or its behaviour.

  Examples: `v1`, `v2`, `internal`.

  - New features that keep existing behaviour compatible need no new version.
    Adding calls, and adding optional (nullable) fields to existing calls, is
    compatible on the wire.
  - One service can serve several versions.
  - A server not used by external customers or partners may support a single
    version, except briefly during migrations.
  - A public server may serve special versions, such as `internal`, that its
    ingress doesn't expose, to keep internal calls private.
  - A public server used by external customers or partners may have
    experimental calls within a stable version, which may change incompatibly
    or be removed. Their action has the `-experimental` suffix until they are
    stable.

## Request path

### Resources are plural

Examples: `products.insert`, `product-groups.insert`.

The exception is a resource that is guaranteed to be one, and only one,
forever: `system.readiness.get`, `orders.cart.insert`, `session.refresh`.

### Actions are verbs

An action always includes a verb. The standard verbs follow.

#### `insert`

Creates a new entity, or adds an entry to a collection or to another entity.

Examples: `products.insert`, `orders.cart.insert`, `product-groups.insert`.

The request may carry a conflict resolution field, `if`. Without it, or
without one of its fields, the server takes the default:

```yaml
# for insert
if:
  exists:
    | replace   # overwrite it: fields the request leaves out are reset to their defaults
    | ignore    # leave it as stored
    | abort     # [default]
    | update {  # upsert: patch it, fields the request leaves out keep their stored values
        fields to patch,  # can be the same struct as for the update action;
                          # missing identity and required fields come from the main body
      }
  others:       # what is stored and the request doesn't list, such as the entity's relations
    | keep      # [default]
    | remove    # what is stored becomes what the request lists
  matches:      # checking this entity; any mismatch => abort
    | exists    # requires it to exist (e.g. for a conditional replace)
    | { edition: number }  # requires it to exist and be at this edition
    | null      # [default] no check
```

- A server needn't support every action, but it must not redefine defaults.
  For example, a server that can only replace a nested collection whole makes
  the client send `if: { exists: replace, others: remove }`, as neither is a
  default. reflectapi's types enforce this.
- Which sets of `exists` / `not_exists` actions an endpoint may offer, and
  each set's default, is fixed by the convention: an endpoint picks one of the
  sets below and can't define its own. The sets are every combination of
  `abort`, `ignore` and `replace`, with `update` or `update` with fields;
  `not_exists` offers only `abort` and `ignore`. A set doesn't name its side:
  the field it fills, `exists` or `not_exists`, does. A set with `abort`
  defaults to it; a set without has no default, so the client always chooses.
  "OrElse" means "or abort, the default, or ignore".

  | Set | Actions | Default | Used on | Idempotent |
  |---|---|---|---|---|
  | `Abort` | abort | abort | any | |
  | `Ignore` | ignore | none | any | ✓ |
  | `AbortOrIgnore` | abort, ignore | abort | any | |
  | `Replace` | replace | none | insert, insert many | ✓ |
  | `ReplaceOrAbort` | replace, abort | abort | insert, insert many | |
  | `ReplaceOrIgnore` | replace, ignore | none | insert, insert many | ✓ |
  | `ReplaceOrElse` | replace, abort, ignore | abort | insert, insert many | |
  | `Update` | update | none | insert, insert many | ✓ |
  | `UpdateOrAbort` | update, abort | abort | insert, insert many | |
  | `UpdateOrIgnore` | update, ignore | none | insert, insert many | ✓ |
  | `UpdateOrElse` | update, abort, ignore | abort | insert, insert many | |
  | `UpdateOrReplace` | update, replace | none | insert, insert many | ✓ |
  | `UpdateOrReplaceOrAbort` | update, replace, abort | abort | insert, insert many | |
  | `UpdateOrReplaceOrIgnore` | update, replace, ignore | none | insert, insert many | ✓ |
  | `UpdateOrReplaceOrElse` | update, replace, abort, ignore | abort | insert, insert many | |
  | `UpdateWith` | update with fields | none | insert | ✓ |
  | `UpdateWithOrAbort` | update with fields, abort | abort | insert | |
  | `UpdateWithOrIgnore` | update with fields, ignore | none | insert | ✓ |
  | `UpdateWithOrElse` | update with fields, abort, ignore | abort | insert | |
  | `UpdateWithOrReplace` | update with fields, replace | none | insert | ✓ |
  | `UpdateWithOrReplaceOrAbort` | update with fields, replace, abort | abort | insert | |
  | `UpdateWithOrReplaceOrIgnore` | update with fields, replace, ignore | none | insert | ✓ |
  | `UpdateWithOrReplaceOrElse` | update with fields, replace, abort, ignore | abort | insert | |

- `others` combines with any `exists` action: after `exists` has decided each
  listed item, `remove` removes what the request doesn't list. Only an
  endpoint whose entity has relations, or an insert of several, offers it.
  `reflectapi::conventions` leaves it out of its clauses, so such an endpoint
  defines its own `if` with `others` beside `exists`.
- `replace` and `update` both insert when the entity doesn't exist; they
  differ in what an existing entity keeps. `replace` overwrites it, so every
  field the request leaves out is reset to its default. `update` patches it,
  so those fields keep their stored values: insert with `exists: update` is
  the upsert. They differ only for fields a request may leave out, so a route
  whose data has none offers one of them, not both.
- An endpoint offers only actions it implements: one that can't replace picks
  a set without `replace`, and never carries out `replace` as `update`.
- A write the server may process more than once, such as from a queue that
  delivers at least once, offers only an idempotent set, one without `abort`,
  and no `{ edition: number }` condition: processing it again then ends the
  same way. Effects of the write, such as events it sends, still happen again
  and need their own deduplication.
- `create`, `add`, `generate` and `new` are not allowed.

<details class="reasoning">
<summary>Reasoning</summary>

- We picked `insert` and banned the rest because:
  - `insert` fits when an item is added to a collection,
  - and also when a client creates an entity and the server inserts it into a
    `resources` collection.
  - A client sometimes creates an entity only partly and lets the server fill
    in the rest, where `create` would be more accurate; we favour simplicity
    over perfect semantics.
  - Insert with `exists: update` is the upsert, so no separate `upsert` verb
    is needed.
  - `products.insert` inserts (adds, creates) a product into `products`.
- An action applies to the resource to the left of the verb:
  - `orders.cart.insert` adds a new cart to a specific (parent) order.
  - `orders.cart.items.insert` inserts a new item into that order's cart.

</details>

#### `update`

A **partial** update of an existing entity.

Examples: `products.update`, `vehicles.update`.

```yaml
# for update
if:
  not_exists:
    | ignore
    | abort     # [default]
  matches:      # checking this entity; any mismatch => abort
    | { edition: number }
    | null      # [default] no check
```

- A server needn't support every action, but it must not redefine defaults.
  For example, a server that can only `ignore` makes the client send
  `if: { not_exists: ignore }`, as it isn't the default. reflectapi's types
  enforce this.
- In the simplest form, the body has the entity's key as required fields, and
  every other field optional:
  - a missing field means no change to it;
  - a present field means set it to the given value, including `null`.

  In Rust, `reflectapi::Option<T>` tells a missing field from `null`.
- When the simplest form isn't enough, for example because an update has
  dependencies between fields and needs several of them, each variant of the
  body must be clearly distinguishable:
  - each variant has a unique set of required fields. This is simpler in the
    schema but costs more to deserialize and identify, so it suits only a few
    variants. In Rust, an enum with `serde(untagged)`; `serde(flatten)` can
    keep only the identifying fields in the enum.
  - or each variant has a unique value of a required field. This is less
    simple in the schema but scales better. In Rust, an enum with
    `serde(tag = "kind")`.
  - or a combination, which takes the benefits of both:

    ```rust,ignore
    #[serde(untagged)]
    enum EntityUpdateRequest {
        Variant1(EntityUpdateRequest1),
        Variant2(EntityUpdateRequest2),
        // ...
        Tagged(TaggedEntityUpdateRequest),
    }

    #[serde(deny_unknown_fields)]
    struct EntityUpdateRequest1 {
        id: uuid::Uuid,
        field1: reflectapi::Option<String>,
        // ...
    }

    #[serde(tag = "kind")]
    enum TaggedEntityUpdateRequest {
        Variant1(SpecificVariantOfEntityUpdate),
        // ...
    }
    ```

#### `remove`

Removes an entity from a collection or from another entity.

```yaml
# for remove
if:
  not_exists:
    | ignore
    | abort     # [default]
  matches:      # checking this entity; any mismatch => abort
    | { edition: number }
    | null      # [default] no check
```

- A server needn't support every action, but it must not redefine defaults,
  as for `update`.
- `delete`, `destroy` and `release` are not allowed.

#### `insert-many`

Inserts several items into a collection, usually a nested one.

Examples: `products.tags.insert-many`, `orders.cart.items.insert-many`.

```yaml
# for insert-many
if:             # at the level of the request: the same rule applies to every item
  exists:
    | replace   # overwrite each listed item that exists (fields it leaves out are reset), insert the rest
    | ignore    # leave each listed item that exists, insert the rest
    | update    # patch each listed item that exists (fields it leaves out are kept), insert the rest (upsert)
    | abort     # [default] if any exists, abort for all
  others:       # items of the collection the request doesn't list
    | keep      # [default]
    | remove    # the collection becomes what the request lists
  parent_matches:  # checking the parent entity; any mismatch => abort
    | exists    # [default] the parent must exist
    | { edition: number }  # the parent must exist and be at this edition
```

#### `update-many`

Updates several items of a collection, usually a nested one.

Examples: `products.tags.update-many`, `orders.cart.items.update-many`.

```yaml
# for update-many
if:             # at the level of the request: the same rule applies to every item
  not_exists:
    | ignore    # update the items that exist, skip the rest
    | abort     # [default] if any doesn't exist, abort for all
  parent_matches:  # checking the parent entity; any mismatch => abort
    | exists    # [default] the parent must exist
    | { edition: number }  # the parent must exist and be at this edition
```

#### `remove-many`

Removes several items from a collection, usually a nested one.

Examples: `products.tags.remove-many`, `orders.cart.items.remove-many`.

```yaml
# for remove-many
if:             # at the level of the request: the same rule applies to every item
  not_exists:
    | ignore    # remove the items that exist, skip the rest
    | abort     # [default] if any doesn't exist, abort for all
  parent_matches:  # checking the parent entity; any mismatch => abort
    | exists    # [default] the parent must exist
    | { edition: number }  # the parent must exist and be at this edition
```

#### `get`

Fetches one item, usually by its primary key or a unique field.

Examples: `products.get`, `users.get`, `stores.get`.

#### `get-many`

Fetches several items, usually by their primary keys or unique fields.

Examples: `products.get-many`, `users.get-many`, `stores.get-many`.

`get-many` and `get` for the same resource return entities with exactly the
same scope and detail: any `get` can be replaced by a `get-many` with one ID.
If `get-many` needs a shallower version, use two resources, one for each:
`listings.get-many` and `listings.details.get`.

#### `list`

Lists entities of one class from one collection.

Examples: `products.variants.list`, `orders.items.list`.

- A list is strongly consistent.
- It can be paginated (see [Lists](#lists)) and can take
  optional filters.

#### `search`

Finds entities by a filter given in the request.

Examples: `products.search`, `vehicles.search`.

- A search always has parameters: a filter or a keyword to search by.
- A search may be eventually consistent.
- Its response can carry aggregations alongside the results.
- It can be paginated (see [Lists](#lists)).
- Different kinds of search on one resource are told apart by a polymorphic
  request, usually with a tag field for the kind.

#### `ingest`

An aggregate, bulk action over one or many entities, of one or several types.
Each entity carries its own action (`insert`, `update` or `remove`) and its
own conflict resolution field.

Example: `catalog.ingest`, with a body such as:

```yaml
products:
  - action: insert | update | remove
    if:         # the conflict resolution field, if applicable
    identity:   # one of the ways to identify the entity uniquely
    # data fields
    price: 123.12
    # ...
  # more products
categories:
  - action: insert | update | remove
    if:
    identity:
    # data fields
    parent: { name: "Tools" }  # one of the ways to identify a related entity
    # ...
```

#### `lookup`

Like `get`, but fetches several unrelated resources, or related ones that
aren't nested (see [Nested resources](#nested-resources)), at once.

Example: `products.lookup`.

#### Other actions

Other actions are allowed for edge or non-trivial cases, but discouraged.

| Don't 👎 | Do 👍 |
|---|---|
| `products.part.link`, `products.link-part` | `products.update {part: part_id}` |
| `products.search-by-part`, `products.search-by-name` | `products.search {part: …}`, `products.search {name: …}` |
| `users.update {password}`, when the flow differs radically from `users.update {name}` | `users.update-password {password, verification_token, …}` |

A custom action, as in the last row, can be better when its steps and flows
are radically different. Don't pack unrelated functionality into one endpoint
just to be consistent; and prefer one `update` for every similar-looking,
related or similarly implemented update.

### Nested resources

Nested resources exist, and are reached, only in the context of a parent.

Examples: `products.variants.list`, `orders.items.list`.

1. A nested collection, a 1⇒N relation, is plural.
2. A nested single entity, a 1⇒1 relation, is singular if it is expected to
   stay 1⇒1 forever: `orders.cart.get`.
3. A nested resource is available, or makes sense, only in the context of its
   parent: not `~~products.part.get~~` but `parts.get`. To include related
   resources that aren't sub-collections, consider a `lookup`.
4. Some resources are nested to group related functionality:
   `system.readiness.get`, `system.liveness.get`,
   `system.background-jobs.pause`.

## Request body

When a call takes arguments:

1. They are in the request body, as an object.
2. When every parameter is optional and none is given, the body is an empty
   object, `{}`. The body may be omitted.
3. A body with one parameter, such as the `id` of a `get`, is still an object:
   `{"id": "…"}`.
4. An entity with one fixed key is identified by a plain field: `id`, or
   `<entity>_id` when the request also names other entities, such as
   `{"id": "…", "new_name": "…"}`. An `identity` field is used instead when an
   entity can be identified in more than one way, its value saying which, such
   as `{"identity": {"sku": "…"}}`, and in `ingest` items, beside `action`,
   `if` and the data fields.
5. Every field and enum value is `snake_case`.
6. A server deserializes a request body from JSON.
7. A performance-sensitive server also deserializes it from MessagePack; a
   client or SDK uses MessagePack when it is available.
8. No URL query or path parameters.

<details class="reasoning">
<summary>Reasoning</summary>

- Without path parameters, which is another departure from traditional REST,
  the implementation is much cleaner: a request's parameters are a single
  struct. It also keeps the conventions usable for APIs that aren't HTTP, such
  as gRPC.
- There is no choice to make between a path, a query, the body or a header.

</details>

## Request headers

1. `content-type`:
   - `application/msgpack` when the body is MessagePack;
   - optionally `application/json` when it is JSON. Without the header, the
     body is JSON.
2. `content-length`: the body's length in bytes, usually set by the HTTP
   client.
3. `authorization`: every authentication value, such as a session token or a
   secret key.
   - These values are **not public**, nor easily derived from public sources.
     They are used for *identification* (who the client, user or application
     is), *authentication* (verifying it is who it says) and possibly
     *authorization* (what it may access).
   - The value can start with a scheme:
     - a session token uses `Bearer`;
     - an integration or other account without a person has no scheme; the key
       itself may carry a prefix saying what kind of account it is and how to
       verify it.
   - No other header is used for authentication.
4. `traceparent`: a client sets it on every request; a server passes it
   downstream and returns it in every response (see
   [W3C Trace Context](https://www.w3.org/TR/trace-context/)).
5. Application-specific headers share one prefix of the application's or
   organisation's choosing, never `x-`. They carry informational context that
   isn't security-sensitive: a value that identifies a caller but can't
   authenticate it, such as a public store key, is fine.
6. No other headers.
7. A server reads header names case-insensitively. Clients send them in
   lower-case `kebab-case`.

## Response status

- `200`: every call that completed, whatever its effects.
- `400`: a request whose body or headers can't be parsed into the call's
  types. The server sends it itself, with a plain-text message.
- `401`: the call needs an identity that can't be verified.
- `403`: the client lacks permission (isn't authorized).
- `404`: the entity doesn't exist.
- `405`: the call doesn't take the method, such as `PUT`. The server sends it
  itself.
- `409`: the write conflicts with what is stored, such as an insert whose
  entity exists, with `exists: abort`.
- `413`: the body is larger than the server accepts. The server or a proxy
  sends it, not the call.
- `415`: the content type isn't JSON or MessagePack. The server sends it
  itself.
- `422`: a well-formed request with bad data, flow or state, such as broken
  referential integrity, or another handled error the client or its user must
  fix.
- `500`: an internal error, a bug the server's owner must fix.
- `503`: the server isn't ready, or a service it depends on is unavailable, so
  it can't respond correctly or completely. A load balancer can return it
  too, typically with an HTML body.
- No other status, including `3xx` redirects: not every client handles them
  correctly and consistently. Redirecting `http` to `https` happens at the
  ingress, and the `http` endpoint isn't part of the API.

`401`, `403`, `404`, `409` and `422` are the call's own errors (see
[Errors](#errors)); the rest come from the server, a proxy or the
infrastructure.

## Response body

1. JSON when the `accept` header is `application/json`, or when it is absent
   and the request body was JSON.
2. MessagePack when the `accept` header is `application/msgpack`, or when it
   is absent and the request body was MessagePack.
3. A successful response is always an object, even when it is empty or has
   one field. An error is the call's error value (see [Errors](#errors)).
4. Every field and enum value is `snake_case`.

### Errors

A call's error is part of its signature, like its input and output: one
error type per call, usually an enum of its failure cases. The body of an
error response is a value of that type, serialized like any output, and each
case chooses its status:

```jsonc
// 404: a case without data
"not_found"

// 409: a case with data
{ "conflict": { "name": "Rex" } }
```

- A client matches on the value, not the status: the value says which case
  it is, and the status tells infrastructure, such as proxies and
  dashboards, what kind of failure it was.
- The responses the server or a proxy sends itself, `400`, `405`, `413` and
  `415`, and every `5xx`, carry a plain-text message, not the call's error. A
  client reports them as failures, not as the call's error.
- An error says nothing security-sensitive, especially on a public server.
  The trace ID travels in the `traceparent` response header, not in the body.

In Rust, reflectapi derives this: see [Errors](../concepts/errors.md).

### Lists

Adapted from Slack's API:

- A cursor-paginated method takes `cursor` and `limit`.
- Without a `cursor`, with or without a `limit`, it returns the first page.
- A paginated response has a top-level `pagination` object, with a
  `next_cursor` *when more results follow*, and possibly a `previous_cursor`
  *when results precede and backward pagination is supported*.
- The next call to the same method passes `next_cursor` as `cursor` for the
  next page, or `previous_cursor` for the previous one.
- An empty, `null` or absent `next_cursor` means no more results.
- An empty, `null` or absent `previous_cursor` means no earlier results, or no
  backward pagination.
- `limit` is the *maximum* number of results per call. Its maximum may change,
  and may differ by method.
- A page can hold *fewer* results than `limit` even when more follow. Don't
  compare the count with the limit to decide whether the results are complete:
  check `next_cursor` (or `previous_cursor`, going backward).
- A paginated response has no total count. Counting costs as much as reading
  everything, paid again on every page, which is the cost cursor pagination
  exists to avoid; and the total is only a snapshot, as items come and go
  while a client pages through. When a client really needs a count, such as
  "showing 20 of 103", the endpoint returns it in its own response object,
  beside the list rather than in `pagination`, and documents its cost. Prefer
  the cheaper forms where they serve:
  - a count on the first page only (the request without `cursor`), kept by
    the client;
  - an estimate, for "about 12,000 results";
  - a capped count, such as counting up to 1,001 and showing "1000+";
  - no count: an empty `next_cursor` already says whether more follow.
- `previous_cursor` follows the same rules as `next_cursor`, backward.
  Backward pagination is rarely needed and isn't implemented until it is; in
  some cases it isn't possible.
- A root-level collection is always wrapped in `items`, even without
  pagination, so pagination can be added later without breaking clients:
  1. add `pagination` to the schema, with the server always returning `null`
     cursors;
  2. update clients to use `pagination`;
  3. only then paginate on the server: limit the page size and return cursors.
- A nested list may be wrapped in `items`, or be a plain list, depending on
  how much it is expected to grow.
  - Turning a plain list into a paginated one breaks compatibility. A list
    that may be paginated later uses the `items: []` form from the start, and
    adds `pagination` when it is implemented, following the steps above.

```jsonc
// a root-level list, not paginated
{
  "items": [...]
  // no "pagination"
}

// a nested list, not paginated, expected to grow
{
  "some_collection": {
    "items": [...]
    // no "pagination"
  }
}

// a nested list, not paginated, not expected to grow
{
  "some_collection": [...]
}

// paginated forward only
{
  "items": [...],
  "pagination": { "next_cursor": "next-page-cursor" }
}

// paginated forward only, last page
{
  "items": [...],
  "pagination": { "next_cursor": null }
}

// paginated both ways
{
  "items": [...],
  "pagination": {
    "previous_cursor": "prev-page-cursor",
    "next_cursor": "next-page-cursor"
  }
}

// paginated both ways, last page
{
  "items": [...],
  "pagination": { "previous_cursor": "prev-page-cursor", "next_cursor": null }
}

// paginated both ways, first page
{
  "items": [...],
  "pagination": { "previous_cursor": null, "next_cursor": "next-page-cursor" }
}
```

### Search results

```jsonc
{
  "payload": {
    "items": [...],
    "pagination": { "next_cursor": "..." }
    // additional fields, such as aggregations or stats
  },

  // lookup-style joined resources the results refer to
  "joined_resources_1": [
    { "entity": { ... }, "nested_relations": { ... } }
  ],
  "joined_resources_2": [ ... ]
}
```

### The root path

A server is often checked by requesting its root URL, while testing or
troubleshooting. To help whoever is troubleshooting, it returns its
application name and any useful debug metadata:

```json
{
  "application": "pets-server",
  "instance-id": 0
}
```

### `openapi-schema.get`

`/api/<version>/openapi-schema.get` returns the server's OpenAPI 3.x schema, as
JSON.

### `system.readiness.get`

`/api/internal/system.readiness.get` returns:

- `200`, with an empty body, when the server is ready for traffic;
- `503`, with a plain-text message saying why, when it isn't.

### `system.liveness.get`

`/api/internal/system.liveness.get` returns:

- `200`, with an empty body, when the server is healthy;
- `503`, with a plain-text message saying why, when it isn't.

### Metrics

A dedicated metrics endpoint, such as one for Prometheus to scrape, is the
exception to this standard: it keeps the path and format its scraper expects.

## Response headers

1. `content-type`:
   - `application/msgpack` when the body is MessagePack;
   - optionally `application/json` when it is JSON.
2. `traceparent`, returned from the request.

## Implementation

- The schema is code first: the server's code defines it.
- Call names, such as `products.list`, appear in the server's code as whole
  strings, so a text search finds a call's implementation.
- The server generates its clients from its schema, such as TypeScript and
  Rust, and publishes them as packages. A package that holds only the
  messages' types and the calls' signatures, without an HTTP client, is better
  named a `protocol` than a `client`; an application can have both, with its
  `client` depending on its `protocol`.
