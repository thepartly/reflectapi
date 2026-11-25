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

## How to read it

The standard is a list of rules. Each rule is a bullet: one statement that
reads alone, closed by the rule's id, such as `api-writes-are-post`. An id
never changes, so a review, a ticket or another document can name a rule by
it, and link to it. What explains a rule is folded beside it: an example,
the reasoning behind it, and how to follow it with reflectapi.

The key words in capitals say how firm a rule is, as in
[RFC 2119](https://www.rfc-editor.org/rfc/rfc2119):

- `MUST` and `MUST NOT`: an absolute requirement. Breaking one fails a
  review.
- `SHOULD` and `SHOULD NOT`: a strong default. Ignoring one needs a real
  reason, not a preference.
- `MAY`: a choice the standard names, and leaves open.

Text without a key word is context, not a rule. The rules name three
parties: the *server*, which serves an API; a *call*, one endpoint of it;
and the *client*, which sends requests.

## Request method

- A call that writes or changes anything MUST be a `POST`.
  `[api-writes-are-post]`
- A server MUST serve a read-only call for both `POST` and `GET`, handling a
  `GET` as it handles a `POST` to the same path.
  `[api-reads-post-and-get]`
- An API's schema MUST list each call once, as a `POST`: the `GET` of a
  read-only call MUST NOT appear in it.
  `[api-schema-lists-post-only]`

<details>
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

<details>
<summary>With reflectapi</summary>

A route marked [`.readonly(true)`](https://docs.rs/reflectapi/latest/reflectapi/struct.RouteBuilder.html#method.readonly) on the builder is served
for `GET` as well as `POST`. The schema records it as read-only, the OpenAPI
document generated from it lists only the route's `post` operation, and the
generated clients always send `POST`. See
[Method](../concepts/wire.md#method).

</details>

## URL

- A call's URL MUST have the form
  `http[s]://<endpoint-address>/api/<version>/<call-name>`.
  `[api-url-form]`

### Versions

- The `version` of a call's URL MUST be the API's major version, or the
  name of a set of calls served apart.
  `[api-version-is-major]`

  <details open>
  <summary>Example</summary>

  `/api/v1/users.get`, `/api/v2/users.get`, `/api/internal/users.get`

  </details>

- A call MUST NOT change incompatibly within a version, in its types or in
  its behaviour: the change MUST be made as a new version of the call, or in
  a new major version of the API.
  `[api-no-breaking-change-in-place]`

- A call added to a version, and an optional (nullable) field added to an
  existing call, MAY be introduced within the version: both are compatible
  on the wire.
  `[api-compatible-changes]`
- A new version of a call MUST be a call of its own, served beside the old
  one, and named as the old one with the suffix `-v2`, then `-v3`, and so
  on.
  `[api-call-version-suffix]`

  <details open>
  <summary>Example</summary>

  `users.invite`, `users.invite-v2`, `users.invite-v3`;
  `organizations.get-v2`; `products.search-v2`

  </details>

- When a new version of a call is introduced, the old one MUST be marked
  deprecated in the API's schema, with a note that names its successor.
  `[api-old-call-version-deprecated]`
- An incompatible change SHOULD be made as new versions of the calls it
  touches, not as a new major version of the API.
  `[api-prefer-call-versions]`

  <details>
  <summary>Reasoning</summary>

  A new version of one call moves only the clients of that call, each when
  it is ready, while a new major version moves every client of the API at
  once. In practice call versions have been enough: an API can evolve
  forever without a second major version.

  </details>

- A new major version of the API SHOULD NOT be introduced for a change that
  keeps compatibility.
  `[api-new-version-only-for-breaking]`
- A server MAY serve several versions at once.
  `[api-several-versions]`
- A server not used by external customers or partners MAY support a single
  version, except briefly during a migration.
  `[api-single-version-when-internal]`
- A public server MAY serve a version that its ingress doesn't expose, to
  keep internal calls private.
  `[api-private-version]`

  <details open>
  <summary>Example</summary>

  `/api/internal/system.readiness.get`

  </details>

- A call within a stable version that can still change incompatibly, or be
  removed, MUST carry the `-experimental` suffix on its action until it is
  stable.
  `[api-experimental-suffix]`

<details>
<summary>With reflectapi</summary>

- A new version of a call is another [route](https://docs.rs/reflectapi/latest/reflectapi/struct.Builder.html#method.route), named with the
  suffix: `.route(users_invite_v2, |b| b.name("users.invite-v2"))`.
- The old route is deprecated on the builder, with
  [`.deprecation_note("Use users.invite-v2.")`](https://docs.rs/reflectapi/latest/reflectapi/struct.RouteBuilder.html#method.deprecation_note), and
  optionally [`.deprecated_since(…)`](https://docs.rs/reflectapi/latest/reflectapi/struct.RouteBuilder.html#method.deprecated_since) and
  [`.sunset(…)`](https://docs.rs/reflectapi/latest/reflectapi/struct.RouteBuilder.html#method.sunset) for its dates. Every generated
  client then marks the old method deprecated, so editors and type checkers
  point its callers to the successor: see
  [Deprecations](../getting-started/deprecations.md).
- A version served apart, such as `internal`, is a builder of its own with
  its own path, [nested](https://docs.rs/reflectapi/latest/reflectapi/struct.Builder.html#method.nest) or served beside the public one: see
  [Organising a Larger API](../concepts/organising.md).

</details>

## Call names

- A call name MUST have the form
  `[category.]resources[.nested-resources].action-verb`, its parts separated
  by `.`.
  `[api-call-name-form]`

  <details open>
  <summary>Example</summary>

  `users.get`, `products.integrations.list`,
  `product-groups.attributes.insert`, `billing.invoices.attachments.list`

  </details>

- A call name MUST have a resource name: its first part, or the part after
  its category.
  `[api-call-name-has-resource]`
- A call name MUST end with an action, even that of a `get` without
  parameters.
  `[api-call-name-has-action]`
- A category, a resource or an action of several words MUST be written in
  `kebab-case`.
  `[api-names-kebab-case]`

<details>
<summary>With reflectapi</summary>

A route's URL is its builder's path and its name:
[`Builder::path("/api/v1")`](https://docs.rs/reflectapi/latest/reflectapi/struct.Builder.html#method.path) and
[`.name("users.get")`](https://docs.rs/reflectapi/latest/reflectapi/struct.RouteBuilder.html#method.name) give `/api/v1/users.get`. The dots of a name become namespaces in the generated
clients, so the call is `client.users.get(…)`: see
[URL](../concepts/wire.md#url) and
[Names become namespaces](../concepts/how-it-works.md#names-become-namespaces).

</details>

### Categories

A category is a classifier in front of a call's resources, for a part of an
API that stands apart from the rest.

<details open>
<summary>Example</summary>

`billing` in `billing.invoices.attachments.list`, `system` in
`system.readiness.get`

</details>

- A call name MAY start with a category.
  `[api-category-optional]`
- An API SHOULD NOT use categories where its resources' names are clear
  without them.
  `[api-avoid-categories]`
- A category MUST NOT follow how the server's implementation is split into
  modules: the calls of one resource MUST share one category, or have none.
  `[api-category-not-implementation]`

  <details open>
  <summary>Example</summary>

  `products.insert` and `products.search`, implemented by two modules, stay
  `products.insert` and `products.search`: not `catalog.products.insert` and
  `search.products.search`.

  </details>

### Resources

- A resource's name MUST be plural, except that of a resource guaranteed to
  stay the only one forever, which MUST be singular.
  `[api-resources-plural]`

  <details open>
  <summary>Example</summary>

  Plural: `products`, `product-groups`. Singular: `session`, an order's
  `cart`.

  </details>

### Nested resources

Nested resources exist, and are reached, only in the context of a parent.

<details open>
<summary>Example</summary>

`products.variants.list`, `orders.items.list`

</details>

- A resource MUST NOT be nested under another unless it exists, or makes
  sense, only in that parent's context.
  `[api-nest-only-dependent-resources]`

  <details open>
  <summary>Example</summary>

  `customers.get`, not `orders.customer.get`: a customer exists apart from
  any order.

  </details>

- A call name MUST NOT nest resources more than two levels deep: a resource,
  and one resource nested under it.
  `[api-two-resource-levels]`

  <details open>
  <summary>Example</summary>

  `shipments.feedback.get` or `orders.shipment-feedback.get`, not
  `orders.shipments.feedback.get`. With a category,
  `billing.invoices.attachments.list` still has two levels.

  </details>

- A nested collection, a 1⇒N relation, MUST be plural, and a nested single
  entity, a 1⇒1 relation expected to stay one forever, MUST be singular.
  `[api-nested-plural-or-singular]`

  <details open>
  <summary>Example</summary>

  `orders.items.list`, `orders.cart.get`

  </details>

### Actions

- An action MUST include a verb.
  `[api-actions-are-verbs]`
- A call SHOULD use one of the standard actions below wherever one fits.
  `[api-prefer-standard-actions]`

#### `insert`

- A call that creates an entity, or adds an entry to a collection or to
  another entity, MUST be named `insert`: `create`, `add`, `generate` and
  `new` MUST NOT be used.
  `[api-insert-not-create]`

  <details open>
  <summary>Example</summary>

  `products.insert`, `orders.cart.insert`, `product-groups.insert`

  </details>

  <details>
  <summary>Reasoning</summary>

  - `insert` fits when an item is added to a collection,
  - and also when a client creates an entity and the server inserts it into
    a `resources` collection.
  - A client sometimes creates an entity only partly and lets the server
    fill in the rest, where `create` would be more accurate; we favour
    simplicity over perfect semantics.
  - [Insert with `exists: update`](#api-replace-overwrites-update-patches)
    is the upsert, so no separate `upsert` verb is needed.

  </details>

- An `insert` MAY take a conflict resolution field, `if`, in its request, with
  the fields and actions [Conflict resolution](#the-if-of-each-action) gives
  for it.
  `[api-insert-if]`

#### `update`

- A call that changes an existing entity MUST be named `update`.
  `[api-update]`

  <details open>
  <summary>Example</summary>

  `products.update`, `vehicles.update`

  </details>

- An `update` MUST be a **partial** update: a field the request leaves out
  stays as stored, and a field it gives is set to the given value, `null`
  included.
  `[api-update-is-partial]`

  <details open>
  <summary>Example</summary>

  For a stored `{"id": "p-1", "name": "Bolt", "notes": "M8"}`:
  `{"id": "p-1", "name": "Bolt M8"}` changes the name and keeps the notes;
  `{"id": "p-1", "notes": null}` clears the notes and keeps the name.

  </details>

  <details>
  <summary>With reflectapi</summary>

  A field of type
  [`reflectapi::Option<T>`](https://docs.rs/reflectapi/latest/reflectapi/enum.Option.html)
  tells a field left out from one set to `null`, and the generated clients
  keep the three states: see
  [Option and reflectapi::Option](../concepts/types.md#option-and-reflectapioption).

  </details>

- The request of an `update` SHOULD have the entity's key as its required
  fields, and every other field optional.
  `[api-update-simple-form]`
- An `update` whose fields only make sense in certain combinations MAY have
  several forms of request, each with its own required fields; a request
  MUST then show which form it is, by a field that names the form or by
  required fields no other form has.
  `[api-update-variants-distinguishable]`

  <details open>
  <summary>Example</summary>

  An `orders.update` with two forms, to reschedule an order or to reassign
  it. A field names the form:

  ```json
  {"id": "o-1", "kind": "reschedule", "date": "2026-12-01", "reason": "customer"}
  {"id": "o-1", "kind": "reassign", "courier_id": "c-7"}
  ```

  Or each form has required fields the other lacks:

  ```json
  {"id": "o-1", "date": "2026-12-01", "reason": "customer"}
  {"id": "o-1", "courier_id": "c-7"}
  ```

  </details>

  <details>
  <summary>Reasoning</summary>

  - A field that names the form scales to many forms: the server reads one
    field to know which form it has, and the schema gives each form a name.
  - Forms told apart by their fields keep requests shorter, but suit only a
    few forms: the server has to try each form in turn, and a request that
    fits none gets a vaguer error.
  - The two can be combined: forms told apart by their fields, with a named
    form among them.

  </details>

  <details>
  <summary>With reflectapi</summary>

  The request is an enum: `#[serde(tag = "kind")]` for forms named by a
  field, `#[serde(untagged)]` for forms told apart by their fields, where
  `#[serde(flatten)]` can keep only the identifying fields in the enum. An
  untagged enum can hold a tagged one as its last variant, to combine them:

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

  </details>

- An `update` MAY take a conflict resolution field, `if`, in its request, with
  the fields and actions [Conflict resolution](#the-if-of-each-action) gives
  for it.
  `[api-update-if]`

#### `remove`

- A call that removes an entity from a collection or from another entity
  MUST be named `remove`: `delete`, `destroy` and `release` MUST NOT be
  used.
  `[api-remove-not-delete]`
- A `remove` MAY take a conflict resolution field, `if`, in its request, with
  the fields and actions [Conflict resolution](#the-if-of-each-action) gives
  for it.
  `[api-remove-if]`

#### `insert-many`, `update-many` and `remove-many`

- A call that inserts, updates or removes several items of one collection,
  usually a nested one, MUST be named `insert-many`, `update-many` or
  `remove-many`.
  `[api-many-actions]`

  <details open>
  <summary>Example</summary>

  `products.tags.insert-many`, `orders.items.update-many`,
  `products.tags.remove-many`

  </details>

- A write of several items MAY take a conflict resolution field, `if`, in its
  request, with the fields and actions [Conflict
  resolution](#the-if-of-each-action) gives for it. Its `parent_matches`
  checks the items' parent, where they have one.
  `[api-many-if]`

#### `get` and `get-many`

- A call that fetches one entity, usually by its primary key or a unique
  field, MUST be named `get`, and one that fetches several that way,
  `get-many`.
  `[api-get-and-get-many]`

  <details open>
  <summary>Example</summary>

  `products.get`, `users.get`, `stores.get-many`

  </details>

- A `get-many` MUST return entities with the same scope and detail as the
  `get` of the same resource, so that a `get` can be replaced by a
  `get-many` of one ID.
  `[api-get-many-same-as-get]`
- Where a shallower version of an entity is needed for a `get-many`, the
  two SHOULD be resources of their own.
  `[api-shallow-version-own-resource]`

  <details open>
  <summary>Example</summary>

  `listings.get-many` and `listings.details.get`

  </details>

#### `list`

- A call that lists the entities of one class from one collection MUST be
  named `list`.
  `[api-list]`

  <details open>
  <summary>Example</summary>

  `products.variants.list`, `orders.items.list`

  </details>

- A `list` MUST be strongly consistent.
  `[api-list-strongly-consistent]`
- A `list` MAY be paginated (see [Lists](#lists)), and MAY take optional
  filters and an order (see [Filters and ordering](#filters-and-ordering)).
  `[api-list-paginated-and-filtered]`

#### `search`

- A call that finds entities by a filter given in the request MUST be named
  `search`.
  `[api-search]`

  <details open>
  <summary>Example</summary>

  `products.search`, `vehicles.search`

  </details>

- A `search` MUST take something to search by, a filter or a keyword (see
  [Filters and ordering](#filters-and-ordering)): it MUST NOT be callable
  without parameters.
  `[api-search-has-parameters]`
- A `search` MAY be eventually consistent.
  `[api-search-eventually-consistent]`
- A `search` MAY be paginated (see [Lists](#lists)), and its response MAY
  carry aggregations beside the results.
  `[api-search-paginated-and-aggregated]`
- Different ways of searching one resource SHOULD be one `search`, told
  apart by the filters a request gives (see
  [Filters and ordering](#filters-and-ordering)), not a call for each way.
  `[api-search-kinds-by-request]`

  <details open>
  <summary>Example</summary>

  `products.search` with `{"filter_by": {"sku": {"exact": "A-1"}}}` or with
  `{"filter_by": {"name": {"matches_words": "bolt m8"}}}`, not
  `products.search-by-sku` and `products.search-by-name`.

  </details>

#### `count`

- A call that returns how many entities a collection has, or how many of
  them match a filter, MUST be named `count`.
  `[api-count]`

  <details open>
  <summary>Example</summary>

  `notifications.count`, and `orders.count` with
  `{"filter_by": {"status": {"any_of": ["open"]}}}`, answered with
  `{"count": 103}`.

  </details>

  <details>
  <summary>Reasoning</summary>

  A count is a call of its own, not a part of a `list` or a `search`:

  - Only the clients that need a count pay for it, and once, not on every
    page.
  - A client asks for the count and the first page at once, and shows the
    page as soon as it arrives.
  - A server can cache, limit and time out a count apart from the list, and
    a slow count never fails the list.
  - It serves a count shown without a list, such as a badge of unread
    notifications.

  </details>

- A `count` SHOULD take the same `filter_by` as the resource's `list` or
  `search` (see [Filters and ordering](#filters-and-ordering)), and MUST
  return the number in a field named `count`.
  `[api-count-request-and-response]`
- A `count` SHOULD return the cheapest form that serves its client, an
  exact number, an estimate or a capped count, and SHOULD say in its
  documentation which one it returns.
  `[api-prefer-cheap-counts]`

  <details open>
  <summary>Example</summary>

  - An estimate: "about 12,000 results".
  - A capped count: counting up to 1,001 and showing "1000+".

  </details>

- A `count` MAY be eventually consistent.
  `[api-count-eventually-consistent]`

#### `ingest` and `lookup`

- A bulk call that writes entities of a resource, each carrying its own
  action, together with the entities of other types they refer to, MUST be
  named `ingest`.
  `[api-ingest]`
- A call that fetches entities by their identities, together with the
  entities of other types they refer to, SHOULD be named `lookup`.
  `[api-lookup]`

Their requests and responses are in
[Referred resources](#referred-resources).

#### Other actions

- An entity whose life is a state machine MAY have an action for each
  transition between its states, named with the transition's verb.
  `[api-state-transition-actions]`

  <details open>
  <summary>Example</summary>

  An order that is placed, confirmed, shipped and closed, or cancelled on
  the way: `orders.confirm`, `orders.ship`, `orders.close`, `orders.cancel`.

  </details>

  <details>
  <summary>Reasoning</summary>

  A transition is more than a change of a status field. Each has its own
  input, its own checks and often its own permissions, and it is allowed
  only from some states. As an action of its own it has a signature that
  says all of that, where an `update` of the status would hide every
  transition behind one call.

  </details>

- An action other than the standard ones MAY be used for a case whose steps
  and flow differ radically from a standard action's.
  `[api-other-actions]`

  <details open>
  <summary>Example</summary>

  `users.invite`, `session.refresh`: neither creates, changes nor removes
  one entity the way a standard action does.

  </details>

- What a call acts on, or searches by, SHOULD be said by its resources and
  its parameters, not by its action.
  `[api-parameters-not-names]`
- Unrelated functionality SHOULD NOT be packed into one call, and updates
  that look alike, are related or are implemented alike SHOULD be one
  `update`.
  `[api-one-call-one-purpose]`

<details open>
<summary>Example</summary>

| Don't 👎 | Do 👍 | Rule |
|---|---|---|
| `products.category.link`, `products.link-category` | `products.update {category: category_id}` | `api-prefer-standard-actions` |
| `products.search-by-category`, `products.search-by-name` | `products.search {filter_by: {category: …}}`, `products.search {filter_by: {name: …}}` | `api-parameters-not-names` |
| `users.update-password` | `users.password.update {password, verification_token, …}` | `api-parameters-not-names` |
| `users.update {password}`, where the flow differs radically from `users.update {name}` | `users.password.update {password, verification_token, …}` | `api-one-call-one-purpose` |
| `orders.update {status: "confirmed"}` | `orders.confirm` | `api-state-transition-actions` |

</details>

## Request body

- A call's arguments MUST be sent in the request body, as one object, even
  when there is a single argument.
  `[api-arguments-in-body-object]`

  <details open>
  <summary>Example</summary>

  `{"id": "…"}`

  </details>

- A request MUST NOT carry arguments in the URL's query or path.
  `[api-no-query-or-path-parameters]`
- Where every argument of a call is optional and none is given, the body
  MUST be the empty object, `{}`, or left out, and a server MUST read a
  body left out as `{}`.
  `[api-empty-body]`
- An entity with one fixed key MUST be identified by a plain field: `id`,
  or `<entity>_id` where the request also names other entities.
  `[api-id-field]`

  <details open>
  <summary>Example</summary>

  `{"id": "…", "new_name": "…"}`, `{"order_id": "…", "product_id": "…"}`

  </details>

- An entity that can be identified in more than one way MUST be identified
  by an `identity` field whose value says which way.
  `[api-identity-field]`

  <details open>
  <summary>Example</summary>

  A product has an id the server gave it, and a SKU its seller knows it by.
  `products.update` takes either, and the request says which:

  ```json
  {"identity": {"id": "8f3c…"}, "price": 0.15}
  {"identity": {"sku": "A-2"}, "price": 0.15}
  ```

  </details>

- Every field name and enum value, of a request and of a response, MUST be
  written in `snake_case`.
  `[api-snake-case]`
- A server MUST accept a request body in JSON.
  `[api-json-requests]`
- A performance-sensitive server SHOULD also accept a request body in
  MessagePack, and a client SHOULD send MessagePack to a server that accepts
  it.
  `[api-msgpack-requests]`

<details>
<summary>Reasoning</summary>

- Without path parameters, which is another departure from traditional REST,
  the implementation is much cleaner: a request's parameters are a single
  struct. It also keeps the conventions usable for APIs that aren't HTTP, such
  as gRPC.
- There is no choice to make between a path, a query, the body or a header.

</details>

<details>
<summary>With reflectapi</summary>

- A route's input is one type that derives [`Input`](https://docs.rs/reflectapi/latest/reflectapi/trait.Input.html),
  the handler's second argument, read from the body. A query string is
  ignored, and a body left out is read as `{}`.
- A route without arguments takes
  [`reflectapi::Empty`](https://docs.rs/reflectapi/latest/reflectapi/struct.Empty.html)
  as its input, which is `{}` on the wire. A route with nothing to return
  gives it as its output too.
- Rust fields are `snake_case` already; an enum takes
  `#[serde(rename_all = "snake_case")]`.
- The [`msgpack` feature](https://docs.rs/crate/reflectapi/latest/features)
  adds MessagePack beside JSON: see
  [Content type](../concepts/wire.md#content-type).
- [`conventions::Identity`](https://docs.rs/reflectapi/latest/reflectapi/conventions/identity/struct.Identity.html)
  and [`IdentityData`](https://docs.rs/reflectapi/latest/reflectapi/conventions/identity/struct.IdentityData.html) are
  the `identity` field as types: see [Identity](./conventions.md#identity).

</details>

### Conflict resolution

A write can say what to do when what it writes already exists, or when what
it changes doesn't: a conflict resolution field, `if`, in its request, beside
the request's own fields. Each item of an [`ingest`](#writing-ingest)
carries one too.

#### The `if` of each action

The fields and actions the `if` of each write can have. A call offers some
or all of them, within the rules below.

For [`insert`](#insert):

```yaml
if:
  exists:
    | replace   # overwrite it: fields the request leaves out are reset to their defaults
    | ignore    # leave it as stored
    | abort     # [default]
    | update {  # upsert: patch it, fields the request leaves out keep their stored values
        fields to patch,  # can be the same struct as for the update action;
                          # missing identity and required fields come from the main body
      }
  not_listed:   # what is stored and the request doesn't list, such as the entity's relations
    | keep      # [default]
    | remove    # what is stored becomes what the request lists
  matches:      # checking this entity; any mismatch => abort
    | exists    # requires it to exist (e.g. for a conditional replace)
    | { edition: number }  # requires it to exist and be at this edition
    | null      # [default] no check
```

For [`update`](#update) and [`remove`](#remove):

```yaml
if:
  not_exists:
    | ignore
    | abort     # [default]
  matches:      # checking this entity; any mismatch => abort
    | { edition: number }
    | null      # [default] no check
```

For [`insert-many`](#insert-many-update-many-and-remove-many):

```yaml
if:
  exists:
    | replace   # overwrite each listed item that exists (fields it leaves out are reset), insert the rest
    | ignore    # leave each listed item that exists, insert the rest
    | update    # patch each listed item that exists (fields it leaves out are kept), insert the rest (upsert)
    | abort     # [default] if any exists, abort for all
  not_listed:   # items of the collection the request doesn't list
    | keep      # [default]
    | remove    # the collection becomes what the request lists
  parent_matches:  # checking the parent entity; any mismatch => abort
    | exists    # [default] the parent must exist
    | { edition: number }  # the parent must exist and be at this edition
```

For [`update-many` and `remove-many`](#insert-many-update-many-and-remove-many):

```yaml
if:
  not_exists:
    | ignore    # update or remove the items that exist, skip the rest
    | abort     # [default] if any doesn't exist, abort for all
  parent_matches:  # checking the parent entity; any mismatch => abort
    | exists    # [default] the parent must exist
    | { edition: number }  # the parent must exist and be at this edition
```

- The `if` of a write of several items MUST apply to the request as a
  whole, the same to every item, and its `abort` MUST abort the write of
  every item when it applies to any.
  `[api-many-if-per-request]`

#### Defaults and action sets

- A server MUST apply the default of an `if` field a request leaves out, and
  of an `if` left out whole: `abort` for `exists` and for `not_exists`, `keep`
  for `not_listed`, no check for `matches`, and `exists` for `parent_matches`.
  `[api-if-defaults-apply]`
- A call MAY offer only some of an `if` field's actions, but MUST NOT change
  a default: a call that doesn't offer a field's default MUST require the
  client to send the field.
  `[api-no-redefined-defaults]`

  <details>
  <summary>Example</summary>

  A server that can only replace a nested collection whole makes the client
  send `if: { exists: replace, not_listed: remove }`, as neither is a
  default. A server that can only `ignore` a missing entity makes the client
  send `if: { not_exists: ignore }`. reflectapi's types enforce this.

  </details>
- A call MUST offer, for `exists` or for `not_exists`, a set of that field's
  own actions, and MUST NOT add an action of its own: for `exists`, any of
  `abort`, `ignore`, `replace` and `update`, plain or with fields; for
  `not_exists`, `abort`, `ignore` or both.
  `[api-action-sets-fixed]`

  <details>
  <summary>With reflectapi</summary>

  [`reflectapi::conventions::conflict::actions`](https://docs.rs/reflectapi/latest/reflectapi/conventions/conflict/actions/index.html)
  has a type for each set, named after its actions: "OrElse" means "or
  abort, the default, or ignore". A set doesn't name its side: the field it
  fills, `exists` or `not_exists`, does.

  | Type | Actions | Default | Used on | Idempotent |
  |---|---|---|---|---|
  | [`Abort`](https://docs.rs/reflectapi/latest/reflectapi/conventions/conflict/actions/enum.Abort.html) | abort | abort | any | |
  | [`Ignore`](https://docs.rs/reflectapi/latest/reflectapi/conventions/conflict/actions/enum.Ignore.html) | ignore | none | any | ✓|
  | [`AbortOrIgnore`](https://docs.rs/reflectapi/latest/reflectapi/conventions/conflict/actions/enum.AbortOrIgnore.html) | abort, ignore | abort | any | |
  | [`Replace`](https://docs.rs/reflectapi/latest/reflectapi/conventions/conflict/actions/enum.Replace.html) | replace | none | insert, insert many | ✓|
  | [`ReplaceOrAbort`](https://docs.rs/reflectapi/latest/reflectapi/conventions/conflict/actions/enum.ReplaceOrAbort.html) | replace, abort | abort | insert, insert many | |
  | [`ReplaceOrIgnore`](https://docs.rs/reflectapi/latest/reflectapi/conventions/conflict/actions/enum.ReplaceOrIgnore.html) | replace, ignore | none | insert, insert many | ✓|
  | [`ReplaceOrElse`](https://docs.rs/reflectapi/latest/reflectapi/conventions/conflict/actions/enum.ReplaceOrElse.html) | replace, abort, ignore | abort | insert, insert many | |
  | [`Update`](https://docs.rs/reflectapi/latest/reflectapi/conventions/conflict/actions/enum.Update.html) | update | none | insert, insert many | ✓|
  | [`UpdateOrAbort`](https://docs.rs/reflectapi/latest/reflectapi/conventions/conflict/actions/enum.UpdateOrAbort.html) | update, abort | abort | insert, insert many | |
  | [`UpdateOrIgnore`](https://docs.rs/reflectapi/latest/reflectapi/conventions/conflict/actions/enum.UpdateOrIgnore.html) | update, ignore | none | insert, insert many | ✓|
  | [`UpdateOrElse`](https://docs.rs/reflectapi/latest/reflectapi/conventions/conflict/actions/enum.UpdateOrElse.html) | update, abort, ignore | abort | insert, insert many | |
  | [`UpdateOrReplace`](https://docs.rs/reflectapi/latest/reflectapi/conventions/conflict/actions/enum.UpdateOrReplace.html) | update, replace | none | insert, insert many | ✓|
  | [`UpdateOrReplaceOrAbort`](https://docs.rs/reflectapi/latest/reflectapi/conventions/conflict/actions/enum.UpdateOrReplaceOrAbort.html) | update, replace, abort | abort | insert, insert many | |
  | [`UpdateOrReplaceOrIgnore`](https://docs.rs/reflectapi/latest/reflectapi/conventions/conflict/actions/enum.UpdateOrReplaceOrIgnore.html) | update, replace, ignore | none | insert, insert many | ✓|
  | [`UpdateOrReplaceOrElse`](https://docs.rs/reflectapi/latest/reflectapi/conventions/conflict/actions/enum.UpdateOrReplaceOrElse.html) | update, replace, abort, ignore | abort | insert, insert many | |
  | [`UpdateWith`](https://docs.rs/reflectapi/latest/reflectapi/conventions/conflict/actions/enum.UpdateWith.html) | update with fields | none | insert | ✓|
  | [`UpdateWithOrAbort`](https://docs.rs/reflectapi/latest/reflectapi/conventions/conflict/actions/enum.UpdateWithOrAbort.html) | update with fields, abort | abort | insert | |
  | [`UpdateWithOrIgnore`](https://docs.rs/reflectapi/latest/reflectapi/conventions/conflict/actions/enum.UpdateWithOrIgnore.html) | update with fields, ignore | none | insert | ✓|
  | [`UpdateWithOrElse`](https://docs.rs/reflectapi/latest/reflectapi/conventions/conflict/actions/enum.UpdateWithOrElse.html) | update with fields, abort, ignore | abort | insert | |
  | [`UpdateWithOrReplace`](https://docs.rs/reflectapi/latest/reflectapi/conventions/conflict/actions/enum.UpdateWithOrReplace.html) | update with fields, replace | none | insert | ✓|
  | [`UpdateWithOrReplaceOrAbort`](https://docs.rs/reflectapi/latest/reflectapi/conventions/conflict/actions/enum.UpdateWithOrReplaceOrAbort.html) | update with fields, replace, abort | abort | insert | |
  | [`UpdateWithOrReplaceOrIgnore`](https://docs.rs/reflectapi/latest/reflectapi/conventions/conflict/actions/enum.UpdateWithOrReplaceOrIgnore.html) | update with fields, replace, ignore | none | insert | ✓|
  | [`UpdateWithOrReplaceOrElse`](https://docs.rs/reflectapi/latest/reflectapi/conventions/conflict/actions/enum.UpdateWithOrReplaceOrElse.html) | update with fields, replace, abort, ignore | abort | insert | |

  [Convention Types](./conventions.md#conflict-resolution) shows a route
  using them, and
  [`reflectapi::conventions`](https://docs.rs/reflectapi/latest/reflectapi/conventions/index.html)
  has the rest of the standard's types.

  </details>
- A call MUST offer only the actions it implements, and MUST NOT carry out
  one action as another, such as `replace` as `update`.
  `[api-only-implemented-actions]`
- A call MUST carry out `replace` as an overwrite, resetting to their
  defaults the fields the request leaves out, and `update` as a patch,
  keeping their stored values; both MUST insert when the entity doesn't
  exist.
  `[api-replace-overwrites-update-patches]`
- A call whose data has no field a request can leave out SHOULD offer
  `replace`, not `update`: every request then carries the whole entity, so
  the two would do the same, and `replace` says what happens.
  `[api-replace-where-nothing-optional]`

#### `not_listed`

- A call MAY offer `not_listed` only where a request can leave something
  stored unlisted: an entity's relations, or the other items of a collection.
  `[api-not-listed-where-it-applies]`
- A server MUST apply `not_listed` after `exists` has decided each listed
  item, whichever action `exists` takes: `remove` then removes what is stored
  and the request doesn't list.
  `[api-not-listed-after-exists]`

#### Writes processed more than once

- A write the server may process more than once, such as from a queue that
  delivers at least once, MUST offer only an idempotent action set, one
  without `abort`, and MUST NOT offer an `{ edition: number }` condition.
  `[api-idempotent-writes]`

  <details>
  <summary>Reasoning</summary>

  Processing the write again then ends the same way. Effects of the write,
  such as events it sends, still happen again and need their own
  deduplication.

  </details>

<details>
<summary>With reflectapi</summary>

- [`reflectapi::conventions::conflict`](https://docs.rs/reflectapi/latest/reflectapi/conventions/conflict/index.html)
  has the `if` clauses and the
  [action sets](https://docs.rs/reflectapi/latest/reflectapi/conventions/conflict/actions/index.html)
  as types: see [Conflict resolution](./conventions.md#conflict-resolution).
- A route takes a clause over one set as its input, so its schema offers
  exactly that set, a request with another action is refused, and a set
  without a default makes `if` required.
- The sets without `abort` implement the
  [`Idempotent`](https://docs.rs/reflectapi/latest/reflectapi/conventions/conflict/actions/trait.Idempotent.html)
  trait. A handler for writes that may come more than once, such as a queue
  consumer, bounds its input by it, `C: Idempotent`, so a set with `abort`
  doesn't compile there.
- The clauses leave `not_listed` out, so a route that offers it defines its
  own `if` type, with `not_listed` beside `exists`.

</details>

### Filters and ordering

A `list` or a `search` narrows and orders its results by two fields of its
request, `filter_by` and `order_by`.

#### Filters

- A call that filters its results MUST take its filters in one field of the
  request, `filter_by`: an object with an optional field for each property
  the call can filter by.
  `[api-filter-by]`
- The filters of a request MUST combine with AND: a result matches every
  filter the request gives, and a filter the request leaves out MUST NOT
  narrow the results.
  `[api-filters-combine-with-and]`
- The value of a filter MUST say how it matches, as one of the ways the call
  offers for that property, and MUST NOT be a bare value.
  `[api-filter-says-how-it-matches]`

  <details open>
  <summary>Example</summary>

  `{"sku": {"starts_with": "A-"}}`, not `{"sku": "A-"}`

  </details>

  <details>
  <summary>Reasoning</summary>

  A bare value leaves the way of matching unsaid: equal, a prefix, any case?
  And it leaves no room to grow: a filter that names its way can gain
  another way later, compatibly, where a bare value would have to change its
  meaning or become a second field.

  </details>

- A filter SHOULD name its way of matching with a word of the table below,
  where one fits.
  `[api-filter-standard-ways]`

  | Way | Value | Matches |
  |---|---|---|
  | `exact` | a value | the property equals the value |
  | `any_of` | a list of values | the property equals any of the values |
  | `starts_with` | a string | the property starts with the string |
  | `matches_words` | a few words, or their beginnings | the property has a word beginning with each one, in any order: `"bol m8"` matches `"M8 bolt, zinc"`, and doesn't match `"M10 bolt"` |
  | `gt`, `gte` | a number or a time | a lower bound: the property is greater than the value, or greater than or equal to it |
  | `lt`, `lte` | a number or a time | an upper bound: the property is less than the value, or less than or equal to it |
  | `all_of` | a list of values | a property with several values has every one of the values |
  | `none_of` | a list of values | a property with several values has none of the values |

- A filter that compares MAY take a lower bound, `gt` or `gte`, and an
  upper bound, `lt` or `lte`, together, as one object; a call chooses which
  bounds its filter offers, and which of them it requires.
  `[api-filter-bounds-together]`

  <details open>
  <summary>Example</summary>

  - A filter that only takes a span requires `gte` and `lt`:
    `{"updated_at": {"gte": "2026-11-01T00:00:00Z", "lt": "2026-12-01T00:00:00Z"}}`
  - A filter with every bound optional takes one end or both:
    `{"quotes": {"gte": 3}}`, `{"price": {"gt": 5, "lte": 10}}`

  </details>

  <details>
  <summary>Reasoning</summary>

  A range needs no way of matching of its own: it is a filter whose type
  has a lower and an upper bound. A call that only needs spans declares two
  required bounds, which is the least to implement; a call that needs more
  declares more bounds, or makes them optional, and its schema says exactly
  which combinations it takes.

  </details>

- A filter for a span, such as a period of time, SHOULD take `gte` and
  `lt`: its start included and its end excluded.
  `[api-span-half-open]`

  <details>
  <summary>Reasoning</summary>

  Half-open spans tile: `[a, b)` then `[b, c)` cover everything once, so a
  client that walks a period in steps neither misses a result nor gets one
  twice.

  </details>

- A filter on a property with several values, such as an entity's tags, MAY
  take `all_of` and `none_of` together, as one object.
  `[api-filter-all-of-none-of]`
- A call that needs OR between filters MAY take it as a field of
  `filter_by` named `or`: a list of objects of filters, all those the call
  offers or some of them, of which a result matches at least one. The
  filters of each object, and `or` with the filters beside it, MUST still
  combine with AND.
  `[api-filter-or]`

  <details open>
  <summary>Example</summary>

  Active jobs with at least three quotes, or with more than ten parts:

  ```json
  {"filter_by": {
    "status": {"any_of": ["active"]},
    "or": [
      {"quotes": {"gte": 3}},
      {"parts": {"gt": 10}}
    ]
  }}
  ```

  </details>

  <details>
  <summary>Reasoning</summary>

  A list of alternatives, each of which combines its own filters with AND,
  can say any combination of AND and OR, and stays readable: nesting an
  `or` inside an `or` adds nothing it can't already say.

  </details>

- A filter on text SHOULD treat case by the way it names: `exact`, `any_of`,
  `all_of` and `none_of` compare values as they are, and `starts_with` and
  `matches_words` ignore case.
  `[api-filter-case-default]`

  <details>
  <summary>Reasoning</summary>

  A reader then knows how a filter treats case without opening its
  documentation. A whole value is mostly sent by a program, which holds it
  as it is stored; a prefix or a few words are mostly typed by a person,
  who doesn't expect case to matter.

  </details>

- A filter on text that treats case differently SHOULD say so in its
  documentation.
  `[api-filter-case-documented]`

  <details open>
  <summary>Example</summary>

  A filter on an email address whose `exact` ignores case, as addresses
  do: `{"email": {"exact": "Ann@Example.com"}}` matches `ann@example.com`.

  </details>

- A call that offers both treatments of case for one property MUST give
  each a way of its own, and name the one that differs from the rule above
  for what it does, such as `exact_ignoring_case`.
  `[api-filter-case-in-name]`

  <details open>
  <summary>Example</summary>

  `{"name": {"exact": "Bolt"}}` matches only `Bolt`;
  `{"name": {"exact_ignoring_case": "bolt"}}` matches `Bolt` and `BOLT`
  too.

  </details>

  <details>
  <summary>Reasoning</summary>

  Here the client chooses, so the choice has to be on the wire:
  documentation can't say which of the two a request wants. A call that
  offers one treatment has nothing for the name to tell apart, so its name
  stays short.

  </details>

#### Ordering

- A call that lets a client choose the order of its results MUST take it in
  one field of the request, `order_by`: an object with the `key` to order
  by, one of those the call offers, and a `direction`, `asc` or `desc`.
  `[api-order-by]`
- A paginated call MUST keep one order through every page of a walk: the one
  `order_by` gives, or the call's own where the request gives none.
  `[api-order-stable-across-pages]`
- A server MUST refuse a request whose `cursor` was issued for a different
  `filter_by` or `order_by` than the request gives: a cursor is bound to the
  filters and the order of the request that got it.
  `[api-cursor-bound-to-query]`

  <details open>
  <summary>Example</summary>

  A walk started with `{"order_by": {"key": "updated_at", "direction":
  "desc"}}` gets a `next_cursor`. Sending that cursor with `"direction":
  "asc"`, or with another `filter_by`, is refused with a `422`.

  </details>

  <details>
  <summary>Reasoning</summary>

  A cursor is a position in one ordered, filtered list. Used on another
  list it points nowhere meaningful, so a walk would skip some results and
  repeat others without any sign of it. Refusing turns that into an error
  the client sees on the first wrong request.

  The request could instead be shaped so that a mismatch can't be sent: a
  first page that states the order, and later pages that carry only a
  cursor. A check at the server is the better trade:

  - That shape guards the order but not the filters, which a request still
    sends beside its page. Guarding both by shape means later pages carry
    nothing but a cursor that holds the whole query.
  - With one shape, a client pages by repeating its request with the cursor
    filled in. With two, the first request and the later ones are built
    differently, in every client and every paging helper.
  - Every field stays optional, so a call can gain an order or pagination
    later without breaking its clients, and a caller that doesn't care
    about the order sends none.

  </details>

  <details>
  <summary>With reflectapi</summary>

  A typed cursor,
  [`Encoded<Cursor>`](https://docs.rs/reflectapi/latest/reflectapi/struct.Encoded.html),
  can carry the order and a digest of the filters beside the position, so
  the handler compares them with the request's and returns a variant of the
  route's error with `#[reflectapi(status = 422)]` when they differ.

  </details>


<details>
<summary>Example</summary>

A request to `products.search`: products whose SKU starts with `A-`, of
either of two brands, tagged `fasteners` but not `internal`, updated since
the start of November, newest first.

```json
{
  "filter_by": {
    "sku": {"starts_with": "A-"},
    "brand": {"any_of": ["b-17", "b-21"]},
    "tag": {"all_of": ["fasteners"], "none_of": ["internal"]},
    "updated_at": {"gte": "2026-11-01T00:00:00Z"}
  },
  "order_by": {"key": "updated_at", "direction": "desc"},
  "page": {"limit": 50}
}
```

</details>

<details>
<summary>With reflectapi</summary>

The filters of the example above:

```rust
use chrono::{DateTime, Utc};

/// `filter_by`: the filters, and `or`, a list of alternatives.
#[derive(Default, serde::Deserialize, reflectapi::Input)]
struct ProductsFilterBy {
    #[serde(flatten)]
    filters: ProductsFilters,
    #[serde(default)]
    or: Vec<ProductsFilters>,
}

/// The filters, declared once, for `filter_by` and for each item of `or`.
/// Every one is optional; an item of `or` has no `or` of its own.
#[derive(Default, serde::Deserialize, reflectapi::Input)]
struct ProductsFilters {
    #[serde(default)]
    sku: Option<SkuFilter>,
    #[serde(default)]
    brand: Option<BrandFilter>,
    #[serde(default)]
    tag: Option<TagFilter>,
    #[serde(default)]
    updated_at: Option<UpdatedAtFilter>,
}

/// One way at a time: an enum of the ways the filter offers.
/// `SkuFilter::StartsWith("A-")` is `{"starts_with": "A-"}` on the wire.
#[derive(serde::Deserialize, reflectapi::Input)]
#[serde(rename_all = "snake_case")]
enum SkuFilter {
    Exact(String),
    StartsWith(String),
}

#[derive(serde::Deserialize, reflectapi::Input)]
#[serde(rename_all = "snake_case")]
enum BrandFilter {
    AnyOf(Vec<String>),
}

/// Ways given together: a struct.
#[derive(serde::Deserialize, reflectapi::Input)]
struct TagFilter {
    #[serde(default)]
    all_of: Vec<String>,
    #[serde(default)]
    none_of: Vec<String>,
}

/// Bounds: a struct of those the filter offers. An `Option` is a bound a
/// request can leave out; a span would have plain `gte` and `lt` fields.
#[derive(serde::Deserialize, reflectapi::Input)]
struct UpdatedAtFilter {
    #[serde(default)]
    gte: Option<DateTime<Utc>>,
    #[serde(default)]
    lt: Option<DateTime<Utc>>,
}
# let filter_by: ProductsFilterBy = serde_json::from_value(serde_json::json!({
#     "sku": {"starts_with": "A-"},
#     "brand": {"any_of": ["b-17", "b-21"]},
#     "tag": {"all_of": ["fasteners"], "none_of": ["internal"]},
#     "updated_at": {"gte": "2026-11-01T00:00:00Z"},
#     "or": [{"sku": {"exact": "A-1"}}, {"tag": {"all_of": ["bolts"]}}]
# })).unwrap();
# assert!(matches!(filter_by.filters.sku, Some(SkuFilter::StartsWith(ref sku)) if sku == "A-"));
# assert!(matches!(filter_by.filters.brand, Some(BrandFilter::AnyOf(ref brands)) if brands.len() == 2));
# assert_eq!(filter_by.filters.tag.as_ref().unwrap().none_of, ["internal"]);
# assert!(filter_by.filters.updated_at.as_ref().unwrap().gte.is_some());
# assert!(filter_by.filters.updated_at.as_ref().unwrap().lt.is_none());
# assert_eq!(filter_by.or.len(), 2);
# assert!(matches!(filter_by.or[0].sku, Some(SkuFilter::Exact(_))));
# assert!(serde_json::from_str::<ProductsFilterBy>(r#"{"sku": {"ends_with": "1"}}"#).is_err());
# let _ = reflectapi::Builder::<()>::new()
#     .route(
#         |_: (), _: ProductsFilterBy, _: reflectapi::Empty| async { reflectapi::Empty {} },
#         |b| b.name("products.search"),
#     )
#     .build()
#     .unwrap();
```

- Every client gets each filter as a typed union, so a way the call doesn't
  offer is a type error, not a request the server refuses.
- Adding a variant to a filter is a compatible change: see
  [Evolving an API](../concepts/evolving.md#adding-enum-variants).

</details>

## Request headers

- A request with a MessagePack body MUST set `content-type` to
  `application/msgpack`; one with a JSON body MAY set it to
  `application/json`, and a server MUST read a body without `content-type`
  as JSON.
  `[api-request-content-type]`
- Every value that authenticates a caller, such as a session token or a
  secret key, MUST be sent in the `authorization` header, and MUST NOT be
  sent in any other header.
  `[api-authorization-header]`

- A value sent in `authorization` MUST NOT be public, nor easily derived
  from public sources.
  `[api-authorization-not-public]`
- A session token in `authorization` MUST use the `Bearer` scheme; the key
  of an integration, or of another account without a person, MUST have no
  scheme, and MAY carry a prefix that says what kind of account it is and
  how to verify it.
  `[api-authorization-schemes]`
- A client SHOULD send `traceparent` with every request (see
  [W3C Trace Context](https://www.w3.org/TR/trace-context/)).
  `[api-traceparent-sent]`
- An application-specific header MUST start with the organisation's prefix,
  the same one in all of its APIs, and MUST NOT start with `x-`.
  `[api-own-header-prefix]`

  <details open>
  <summary>Example</summary>

  An organisation named Example sends `example-store-key` and
  `example-client-version`, in every one of its APIs.

  </details>

- An application-specific header MUST NOT carry a security-sensitive value;
  it MAY carry one that identifies a caller without authenticating it, such
  as a public store key.
  `[api-own-headers-not-sensitive]`
- A call MUST NOT take input from a header its client sets, other than
  `content-type`, `content-length`, `accept`, `authorization`,
  `traceparent` and the application-specific ones.
  `[api-no-other-input-headers]`

  <details>
  <summary>Reasoning</summary>

  A call's input is its body: one place, typed, and in the schema. The
  headers say what the body can't say of itself: its format, who sends it,
  and the trace it belongs to. An input in a header is easy to miss when
  writing a client, and proxies between the client and the server may drop
  it or change it.

  </details>

- A server MAY read a header that the network between the client and the
  server sets, such as the client's address from a proxy or an identity
  from a private network, and a header that HTTP clients send on their own,
  such as `origin` and `user-agent`.
  `[api-network-headers]`

  <details open>
  <summary>Example</summary>

  `x-forwarded-for` from a load balancer, `cf-connecting-ip` from
  Cloudflare, `tailscale-user-login` from Tailscale.

  </details>

  <details>
  <summary>Reasoning</summary>

  These aren't inputs of the call: no client's code writes them. They say
  how a request arrived, which a server needs to limit rates, to keep an
  audit record and to check access.

  An identity the network vouches for isn't a value the client sends to
  authenticate, so the rule on `authorization` above still holds for
  everything a client sends.

  </details>

- A server MUST rely on a header the network sets only where every request
  reaches it through the component that sets the header, and that component
  replaces any value the client sent.
  `[api-network-headers-trusted]`

  <details>
  <summary>Reasoning</summary>

  A client can send any header. Where a request can reach the server around
  the proxy, or the proxy passes the client's value through, the header
  says what the client wants it to say: a forged address gets past a rate
  limit, and a forged identity gets past an access check.

  </details>

- A server MUST read header names without regard to case, and a client
  SHOULD send them in lower-case `kebab-case`.
  `[api-header-names-case]`

<details>
<summary>With reflectapi</summary>

A route declares the headers it reads as a type that derives
[`Input`](https://docs.rs/reflectapi/latest/reflectapi/trait.Input.html), the handler's third argument, so they are in
the schema and typed in every client: see
[Request Headers](../clients/index.md#request-headers). A header the network
sets is a field of that type with `#[serde(default)]`, since a client doesn't
send it.

</details>

## Response status

- A server MUST answer a call that completed with `200`, whatever the
  call's effects.
  `[api-200-for-success]`
- Each case of a call's error type, the failures the call declares in its
  signature (see [Errors](#errors)), MUST answer with one status, among
  `401`, `403`, `404`, `409` and `422`, as the table below gives them.
  `[api-error-statuses]`
- A server MUST answer `400` to a request whose body or headers can't be
  parsed into the call's types, `405` to a method the call doesn't take, and
  `415` to a content type that is neither JSON nor MessagePack; a case of a
  call's error type MUST NOT use these, nor `413`.
  `[api-server-statuses]`
- A server MUST answer `500` to an internal error, and `503` when it isn't
  ready, or when a service it depends on is unavailable, so that it can't
  answer correctly or completely.
  `[api-500-and-503]`
- A server MUST NOT answer with any other status, a `3xx` redirect
  included.
  `[api-no-other-statuses]`

  <details>
  <summary>Reasoning</summary>

  Not every client handles redirects correctly and consistently.
  Redirecting `http` to `https` happens at the ingress, and the `http`
  endpoint isn't part of the API.

  </details>

| Status | When | Body |
|---|---|---|
| `200` | The call completed, whatever its effects | the call's output |
| `400` | The body or the headers can't be parsed into the call's types | a message from the server |
| `401` | The call needs an identity that can't be verified | a value of the call's error type |
| `403` | The client lacks permission (isn't authorized) | a value of the call's error type |
| `404` | The entity doesn't exist | a value of the call's error type |
| `405` | The call doesn't take the method, such as `PUT` | a message from the server |
| `409` | The write conflicts with what is stored, such as an insert whose entity exists, with `exists: abort` | a value of the call's error type |
| `413` | The body is larger than the server accepts | a message from the server, or from a proxy |
| `415` | The content type is neither JSON nor MessagePack | a message from the server |
| `422` | A well-formed request with bad data, flow or state, such as broken referential integrity, or another handled error the client or its user has to fix | a value of the call's error type |
| `500` | An internal error, a bug for the server's owner to fix | a message from the server |
| `503` | The server isn't ready, or a service it depends on is unavailable | a message from the server, or from a load balancer, typically HTML |

<details>
<summary>With reflectapi</summary>

- A route's error derives [`ResponseError`](https://docs.rs/reflectapi/latest/reflectapi/derive.ResponseError.html),
  and each variant gives its status, `#[reflectapi(status = 404)]`: see
  [A route's error](../concepts/errors.md#a-routes-error).
- The derive refuses, as it compiles, the statuses the server sends itself,
  such as `400`, `405` and `415`, and every `5xx` but `500` on an
  `InternalError` variant. It doesn't limit a route's error to the five
  statuses above: that part is left to review.
- reflectapi sends `400`, `405` and `415` itself: see
  [Statuses reflectapi sends](../concepts/wire.md#statuses-reflectapi-sends).

</details>

## Response body

- A server MUST answer in the format the request's `accept` header names,
  `application/json` or `application/msgpack`, and without that header, in
  the format of the request's body.
  `[api-response-format]`

  <details>
  <summary>With reflectapi</summary>

  reflectapi answers in the format of the request's body, and ignores
  `accept`: see [Content type](../concepts/wire.md#content-type).

  </details>

- The body of a successful response MUST be an object, even when it is
  empty or has one field.
  `[api-response-is-object]`

### Errors

- A call that can fail MUST have one error type, part of its signature like
  its input and its output, and the body of its error response MUST be a
  value of that type, serialized as an output is.
  `[api-typed-errors]`

  <details>
  <summary>Example</summary>

  The type is usually an enum of the call's failure cases, each choosing its
  status:

  ```jsonc
  // 404: a case without data
  "not_found"

  // 409: a case with data
  { "conflict": { "name": "Rex" } }
  ```

  </details>

- A client MUST tell the cases of a call's error type by the value in the
  body, not by the status.
  `[api-match-error-value]`

  <details>
  <summary>Reasoning</summary>

  The value says which case it is. The status tells infrastructure, such as
  proxies and dashboards, what kind of failure it was.

  </details>

- A client MUST report a `400`, `405`, `413`, `415` or `5xx` response as a
  failure of the request, not as a value of the call's error type.
  `[api-failures-not-errors]`

  <details>
  <summary>Reasoning</summary>

  The server, or a proxy before it, sends these itself, with a plain-text
  message, not a value of the call's error type.

  </details>

- An error MUST NOT carry security-sensitive information.
  `[api-errors-not-sensitive]`
- The body of an error SHOULD NOT carry a trace ID: the `traceparent`
  response header carries it.
  `[api-trace-id-in-header]`

<details>
<summary>With reflectapi</summary>

A handler returns `Result<Output, Error>`, and the error type is in the
schema. Every generated client gives the caller the route's error as a typed
value, kept apart from network and server failures: see
[Errors](../concepts/errors.md).

</details>

### Lists

- A paginated call MUST take the page a request asks for in one field of
  the request, `page`, with a `cursor` and a `limit`, both optional, and
  MUST return the first page to a request without a `cursor`.
  `[api-cursor-and-limit]`

  <details open>
  <summary>Example</summary>

  `{"page": {"limit": 50}}`, then `{"page": {"cursor": "eyJ…", "limit": 50}}`

  </details>

- A paginated response MUST have a top-level `pagination` object, whose
  `next_cursor` is set while more results follow, and is empty, `null` or
  absent when none do.
  `[api-next-cursor]`
- A client MUST ask for the next page by sending the last response's
  `next_cursor` as the `cursor` of the `page` of its next request to the
  same call.
  `[api-next-page-by-cursor]`
- A server MUST read `limit` as the largest number of results to return; it
  MAY return fewer, even while more follow, and MAY change the largest
  `limit` it accepts, for each call.
  `[api-limit-is-a-maximum]`
- A client MUST tell whether more results follow by `next_cursor`, and MUST
  NOT tell it by comparing the number of results with `limit`.
  `[api-end-by-cursor-not-count]`
- A paginated response SHOULD NOT carry a total count of the results: a
  client that needs one calls the resource's `count` (see
  [`count`](#count)).
  `[api-no-total-count]`

  <details>
  <summary>Reasoning</summary>

  Counting costs as much as reading everything, paid again on every page,
  which is the cost cursor pagination exists to avoid; and the total is only
  a snapshot, as items come and go while a client pages through.

  </details>

- Backward pagination SHOULD NOT be implemented until it is needed; where
  it is, `pagination` MUST carry a `previous_cursor`, which follows the
  rules of `next_cursor` in the other direction.
  `[api-backward-pagination]`

  <details>
  <summary>Reasoning</summary>

  Backward pagination is rarely needed, and in some cases isn't possible. An
  empty, `null` or absent `previous_cursor` means no earlier results, or no
  backward pagination.

  </details>

- A collection at the root of a response MUST be wrapped in `items`, whether
  it is paginated or not.
  `[api-root-list-in-items]`

  <details>
  <summary>Reasoning</summary>

  Pagination can then be added later without breaking clients:

  1. add `pagination` to the schema, with the server always returning `null`
     cursors;
  2. update clients to use `pagination`;
  3. only then paginate on the server: limit the page size and return
     cursors.

  </details>

- A nested list MAY be a plain list where it isn't expected to grow, and
  SHOULD be wrapped in `items` where it may be paginated later.
  `[api-nested-list-in-items]`

  <details>
  <summary>Reasoning</summary>

  Turning a plain list into a paginated one breaks compatibility. A list
  that may be paginated later uses the `items: []` form from the start, and
  adds `pagination` when it is implemented, following the steps above. When
  in doubt, wrap the list in `items`.

  </details>

<details>
<summary>Example</summary>

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

</details>

<details>
<summary>Reasoning</summary>

Adapted from Slack's API standard. A cursor marks a position in the list, so
a walk through the pages stays correct while items come and go, and every
page costs the same. With page numbers or offsets, a walk skips or repeats
items when the list changes, and each later page costs more.

</details>

<details>
<summary>With reflectapi</summary>

[`reflectapi::conventions`](https://docs.rs/reflectapi/latest/reflectapi/conventions/index.html) has
[`PageRequest`](https://docs.rs/reflectapi/latest/reflectapi/conventions/pagination/struct.PageRequest.html) (`cursor`,
`limit`), the request's `page` field,
[`ForwardPaginatedList`](https://docs.rs/reflectapi/latest/reflectapi/conventions/pagination/struct.ForwardPaginatedList.html)
and
[`NonPaginatedList`](https://docs.rs/reflectapi/latest/reflectapi/conventions/pagination/struct.NonPaginatedList.html)
as types. A cursor can be typed on the server, as
[`Encoded<Cursor>`](https://docs.rs/reflectapi/latest/reflectapi/struct.Encoded.html), while clients still see a
string: see [Pagination](./conventions.md#pagination).

</details>

## Response headers

- A response with a MessagePack body MUST set `content-type` to
  `application/msgpack`, and one with a JSON body MAY set it to
  `application/json`.
  `[api-response-content-type]`
- A server MUST return a request's `traceparent` with its response, and
  SHOULD pass it on to the services it calls.
  `[api-traceparent-returned]`

<details>
<summary>With reflectapi</summary>

reflectapi returns the request's `traceparent` with a success and with a
route's error: see [Response headers](../concepts/wire.md#response-headers).

</details>

## Referred resources

Entities refer to entities of other types: a product to its brand, an order
to its customer. Writing or reading them one type at a time costs a call for
each type, and embedding each referred entity in its referrer repeats it in
every referrer. So a call that writes or reads entities with the ones they
refer to carries all of them in one body: the call's own entities under
`items`, each type they refer to in a list of its own, and each entity once.
`ingest` writes this way; `lookup` and `search` read this way.

- A body that carries entities of several types MUST hold the entities of
  the call's own resource in a list under `items`, and the entities of each
  type they refer to in a list of their own, under a field named after the
  type.
  `[api-referred-lists-by-type]`

  <details open>
  <summary>Example</summary>

  A body of `products.ingest`, `products.lookup` or `products.search`, with
  the brands its products refer to:

  ```json
  {
    "items": [
      {"sku": "A-1", "name": "Bolt M8", "brand_id": "b-17"},
      {"sku": "A-2", "name": "Bolt M10", "brand_id": "b-17"}
    ],
    "brands": [
      {"id": "b-17", "name": "Acme", "country": "NZ"}
    ]
  }
  ```

  </details>

  <details>
  <summary>Reasoning</summary>

  `items` is where every list of a call's own entities is (see
  [Lists](#lists)), so a client reads the results of a `list`, a `search`
  and a `lookup` the same way, and pagination can be added beside them.

  </details>

- In a body that carries entities of several types, an entity MUST refer to
  an entity of another type by that entity's identity, and MUST NOT embed a
  copy of it.
  `[api-refer-by-identity]`

  <details open>
  <summary>Example</summary>

  A product refers to its brand:

  ```json
  {
    "sku": "A-1",
    "name": "Bolt M8",
    "brand_id": "b-17"
  }
  ```

  It does **NOT** embed it:

  ```json
  {
    "sku": "A-1",
    "name": "Bolt M8",
    "brand": {"id": "b-17", "name": "Acme", "country": "NZ"}
  }
  ```

  </details>

- A body that carries entities of several types MUST hold each entity once,
  however many others refer to it.
  `[api-referred-once]`

  <details open>
  <summary>Example</summary>

  Two products of one brand come with that brand once:

  ```json
  {
    "items": [
      {"sku": "A-1", "brand_id": "b-17"},
      {"sku": "A-2", "brand_id": "b-17"}
    ],
    "brands": [
      {"id": "b-17", "name": "Acme"}
    ]
  }
  ```

  </details>

<details>
<summary>Reasoning</summary>

- A thousand products of one brand carry that brand once, not a thousand
  times: bodies stay small, and a client stores what it reads without
  removing duplicates first.
- One call replaces a call for each type, so the entities a client gets are
  consistent with each other, and a write of related entities can succeed or
  fail as a whole.
- A reference by identity reads the same in a request and in a response, so
  what a client looked up it can send back to ingest.

</details>

### Writing: `ingest`

- Each item of an `ingest` MUST carry its `action` (`insert`, `update` or
  `remove`), its `identity` and, where the action takes one, its own
  [`if`](#conflict-resolution), beside its data fields.
  `[api-ingest-items]`
- An item of an `ingest` MAY carry its relations, the entities nested under
  it, as lists of items within it, each relation item with its own `action`,
  `identity` and `if`.
  `[api-ingest-relations-within-item]`
- The response of an `ingest` MUST report a result for each item, and for
  each relation item, in the request's lists and order: the item's identity,
  with either what was written or the issue that stopped it.
  `[api-ingest-result-per-item]`
- An `ingest` SHOULD be atomic by default, writing every item or none, and
  MAY offer a mode in which each item succeeds or fails on its own.
  `[api-ingest-atomic-by-default]`

<details>
<summary>Example</summary>

A request to `products.ingest`: two products refer to one brand, which the
request writes once.

```yaml
items:
  - action: insert
    if: { exists: update }
    identity: { sku: "A-1" }
    name: "Bolt M8"
    price: 0.12
    brand: { name: "Acme" }         # a reference, by identity
    tags:                           # a relation: the product's own items
      - action: insert
        if: { exists: ignore }
        identity: { name: "fasteners" }
  - action: update
    identity: { sku: "A-2" }
    price: 0.15
    brand: { name: "Acme" }         # the same brand, referred to again
brands:
  - action: insert
    if: { exists: ignore }
    identity: { name: "Acme" }
    country: "NZ"
```

Its response, in the non-atomic mode: a result for each item, in the same
lists and order.

```yaml
items:
  - identity: { sku: "A-1" }
    success: { id: "8f3c…", edition: 4 }
    tags:
      - identity: { name: "fasteners" }
        success: {}
  - identity: { sku: "A-2" }
    issue: not_found
brands:
  - identity: { name: "Acme" }
    success: { id: "b-17", edition: 1 }
```

</details>

### Reading: `include`

A `lookup` or a `search` returns its own entities. A request can ask for
more with them: the entities they refer to, such as the brands of the
products, and their relations, such as each product's tags.

- A `lookup` or a `search` MUST return referred entities and relations only
  when the request asks for them.
  `[api-include-on-request]`

  <details>
  <summary>Reasoning</summary>

  Each of them costs the server a query and the client a larger response. A
  client that doesn't need the brands doesn't pay for them.

  </details>

- A request MUST ask for them in one field, `include`: an object with an
  optional field for each thing the call can add to its response.
  `[api-include-field]`

  <details open>
  <summary>Example</summary>

  The products alone:

  ```json
  {"filter_by": {"name": {"matches_words": "bolt"}}}
  ```

  The products, with their brands and each product's tags:

  ```json
  {
    "filter_by": {"name": {"matches_words": "bolt"}},
    "include": {"brands": {}, "tags": {}}
  }
  ```

  </details>

  <details>
  <summary>Reasoning</summary>

  The value of each field is an object, empty at first, so that it can gain
  options later, such as how many levels of a tree to return, without
  breaking clients. A list of names, `["brands", "tags"]`, would have no
  room for them.

  </details>

- A response MUST hold, of a referred type, only the entities that its
  `items` refer to.
  `[api-include-only-referred]`

### Reading: `lookup`

- The request of a `lookup` MUST name the entities to fetch in one field,
  `identities`.
  `[api-lookup-identities]`
- The `items` of a `lookup`'s response SHOULD have an entry for each
  identity the request names, in the request's order: the identity, with
  the entity, or `null` where it wasn't found.
  `[api-lookup-entry-per-identity]`

<details>
<summary>Example</summary>

A request to `products.lookup`: three products, with their tags and the
brands they refer to.

```yaml
identities: [{ sku: "A-1" }, { sku: "A-2" }, { sku: "A-9" }]
include:
  tags: {}            # each product's tags
  brands: {}          # the brands the products refer to
```

Its response: the brand appears once, though two products refer to it.

```yaml
items:
  - identity: { sku: "A-1" }
    entity: { id: "8f3c…", name: "Bolt M8", brand_id: "b-17" }
    tags: [{ name: "fasteners" }]
  - identity: { sku: "A-2" }
    entity: { id: "9a1d…", name: "Bolt M10", brand_id: "b-17" }
    tags: []
  - identity: { sku: "A-9" }
    entity: null      # not found
brands:
  - { id: "b-17", name: "Acme", country: "NZ" }
```

</details>

### Reading: `search`

- The response of a `search` MUST hold its results in `items` at its root,
  with `pagination` and the lists of referred entities beside them.
  `[api-search-response-shape]`

  <details>
  <summary>Example</summary>

  A request to `products.search`, asking for the brands the results refer
  to:

  ```json
  {
    "filter_by": {"name": {"matches_words": "bolt"}},
    "include": {"brands": {}},
    "page": {"limit": 2}
  }
  ```

  Its response: both results refer to one brand, which appears once.

  ```json
  {
    "items": [
      {"id": "8f3c…", "sku": "A-1", "name": "Bolt M8", "brand_id": "b-17"},
      {"id": "9a1d…", "sku": "A-2", "name": "Bolt M10", "brand_id": "b-17"}
    ],
    "brands": [
      {"id": "b-17", "name": "Acme", "country": "NZ"}
    ],
    "pagination": {"next_cursor": "eyJ…"}
  }
  ```

  </details>

<details>
<summary>With reflectapi</summary>

- An item's `action` is a tagged enum, `#[serde(tag = "action")]`, whose
  variants are the write clauses of
  [`reflectapi::conventions::conflict`](https://docs.rs/reflectapi/latest/reflectapi/conventions/conflict/index.html)
  over [`IdentityData`](https://docs.rs/reflectapi/latest/reflectapi/conventions/identity/struct.IdentityData.html):
  each item then has its own `if`, `identity` and data.
- A reference is a field of the referred type's
  [`Identity`](https://docs.rs/reflectapi/latest/reflectapi/conventions/identity/struct.Identity.html)
  type.
- A field of `include` is a
  [`reflectapi::Option`](https://docs.rs/reflectapi/latest/reflectapi/enum.Option.html)
  of its options, so a request that leaves it out differs from one that
  sends `{}`.
- Generic item and result types stay generic in every client, so one
  definition serves every entity type.

</details>

## Service calls

- A server SHOULD answer a request to its root URL with its application's
  name, and MAY add debug metadata.
  `[api-root-path]`

  <details>
  <summary>Example</summary>

  A server is often checked by requesting its root URL, while testing or
  troubleshooting, and the answer helps whoever is troubleshooting:

  ```json
  {
    "application": "pets-server",
    "instance_id": 0
  }
  ```

  </details>

### Schemas

- A server SHOULD serve its API's schema as JSON: the reflectapi schema at
  `/api/<version>/system.schemas.reflectapi.get`, where it is built with
  reflectapi, and the OpenAPI 3.x document at
  `/api/<version>/system.schemas.openapi.get`.
  `[api-schema-calls]`

  <details>
  <summary>Reasoning</summary>

  The two are named as any other call is: in the `system` category, beside
  readiness and liveness, with a nested resource for each format. Each is
  then a read-only call without parameters, which a browser, a client
  generator or an API explorer fetches by its URL.

  </details>

<details>
<summary>With reflectapi</summary>

[`Builder::build`](https://docs.rs/reflectapi/latest/reflectapi/struct.Builder.html#method.build) returns the schema beside the routes, to write
out for client generation or to serve, as it is or as an OpenAPI document
([`codegen::openapi::Spec`](https://docs.rs/reflectapi/latest/reflectapi/codegen/openapi/struct.Spec.html)): see
[The schema](../serving/axum.md#the-schema).

</details>

### Readiness and liveness

- A server SHOULD serve `/api/internal/system.readiness.get`, answering
  `200` with an empty object when it is ready for traffic, and `503` with a
  plain-text message saying why when it isn't.
  `[api-readiness]`
- A server SHOULD serve `/api/internal/system.liveness.get`, answering
  `200` with an empty object when it is healthy, and `503` with a
  plain-text message saying why when it isn't.
  `[api-liveness]`

## Implementation

- An API's schema SHOULD be code first: defined by the server's code, not
  written apart from it.
  `[api-code-first]`
- A call's name SHOULD appear in the server's code as one whole string, so
  that a text search finds the call's implementation.
  `[api-call-names-whole-strings]`

  <details open>
  <summary>Example</summary>

  `"products.list"`, not `"products." + action`

  </details>

- A client that a server's owner provides, such as for TypeScript or Rust,
  MUST be generated from the server's schema, not written by hand.
  `[api-generated-clients]`

  <details>
  <summary>Reasoning</summary>

  A client written by hand drifts from its server: a field is renamed on
  one side, an error case is added on the other, and nothing fails until a
  request does. A generated client changes with the schema, and a change
  that breaks a caller fails where the caller is compiled.

  </details>

- A server's owner SHOULD publish its generated clients as packages, where
  they are used outside the server's own repository.
  `[api-published-clients]`

<details>
<summary>With reflectapi</summary>

- The schema is code first by construction: it is reflected from the
  handlers' signatures and their types.
- A route's name is one string on the builder,
  [`.name("products.list")`](https://docs.rs/reflectapi/latest/reflectapi/struct.RouteBuilder.html#method.name).
- `reflectapi codegen` writes the TypeScript, Python and Rust clients from
  the schema: see [Generated Clients](../clients/index.md).

</details>

## Exceptions

A dedicated metrics endpoint, such as one for Prometheus to scrape, keeps
the path and format its scraper expects.
