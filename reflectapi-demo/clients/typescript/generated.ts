// DO NOT MODIFY THIS FILE MANUALLY
// This file was generated by reflectapi-cli
//
// Schema name: Demo application
// This is a demo application

export function client(base: string | Client): __definition.Interface {
  return __implementation.__client(base);
}
/* <----- */

export interface Client {
  request(
    path: string,
    body: string,
    headers: Record<string, string>,
  ): Promise<[number, string]>;
}

export type AsyncResult<T, E> = Promise<Result<T, Err<E>>>;

export type FixedSizeArray<T, N extends number> = Array<T> & { length: N };

export class Result<T, E> {
  constructor(private value: { ok: T } | { err: E }) {}

  public ok(): T | undefined {
    if ("ok" in this.value) {
      return this.value.ok;
    }
    return undefined;
  }
  public err(): E | undefined {
    if ("err" in this.value) {
      return this.value.err;
    }
    return undefined;
  }

  public is_ok(): boolean {
    return "ok" in this.value;
  }
  public is_err(): boolean {
    return "err" in this.value;
  }

  public map<U>(f: (r: T) => U): Result<U, E> {
    if ("ok" in this.value) {
      return new Result({ ok: f(this.value.ok) });
    } else {
      return new Result({ err: this.value.err });
    }
  }
  public map_err<U>(f: (r: E) => U): Result<T, U> {
    if ("err" in this.value) {
      return new Result({ err: f(this.value.err) });
    } else {
      return new Result({ ok: this.value.ok });
    }
  }

  public unwrap_ok(): T {
    if ("ok" in this.value) {
      return this.value.ok;
    }
    throw new Error(
      `called \`unwrap_ok\` on an \`err\` value: ${this.value.err}`,
    );
  }
  public unwrap_err(): E {
    if ("err" in this.value) {
      return this.value.err;
    }
    throw new Error("called `unwrap_err` on an `ok` value");
  }

  public unwrap_ok_or_default(default_: T): T {
    if ("ok" in this.value) {
      return this.value.ok;
    }
    return default_;
  }
  public unwrap_err_or_default(default_: E): E {
    if ("err" in this.value) {
      return this.value.err;
    }
    return default_;
  }

  public unwrap_ok_or_else(f: (e: E) => T): T {
    if ("ok" in this.value) {
      return this.value.ok;
    }
    return f(this.value.err);
  }
  public unwrap_err_or_else(f: (v: T) => E): E {
    if ("err" in this.value) {
      return this.value.err;
    }
    return f(this.value.ok);
  }

  public toString(): string {
    if ("ok" in this.value) {
      return `Ok { ok: ${this.value.ok} }`;
    } else {
      return `Err { err: ${this.value.err} }`;
    }
  }
}

export class Err<E> {
  constructor(private value: { application_err: E } | { other_err: any }) {}

  public err(): E | undefined {
    if ("application_err" in this.value) {
      return this.value.application_err;
    }
    return undefined;
  }
  public other_err(): any | undefined {
    if ("other_err" in this.value) {
      return this.value.other_err;
    }
    return undefined;
  }

  public is_err(): boolean {
    return "application_err" in this.value;
  }
  public is_other_err(): boolean {
    return "other_err" in this.value;
  }

  public map<U>(f: (r: E) => U): Err<U> {
    if ("application_err" in this.value) {
      return new Err({ application_err: f(this.value.application_err) });
    } else {
      return new Err({ other_err: this.value.other_err });
    }
  }
  public unwrap(): E {
    if ("application_err" in this.value) {
      return this.value.application_err;
    } else {
      throw this.value.other_err;
    }
  }
  public unwrap_or_default(default_: E): E {
    if ("application_err" in this.value) {
      return this.value.application_err;
    }
    return default_;
  }
  public unwrap_or_else(f: () => E): E {
    if ("application_err" in this.value) {
      return this.value.application_err;
    }
    return f();
  }

  public toString(): string {
    if ("application_err" in this.value) {
      return `Application Error: ${this.value.application_err}`;
    } else {
      return `Other Error: ${this.value.other_err}`;
    }
  }
}

/* -----> */

export namespace __definition {
  export interface Interface {
    health: health.Interface;
    pets: pets.Interface;
  }

  export namespace health {
    export interface Interface {
      /// Check the health of the service
      check: (input: {}, headers: {}) => AsyncResult<{}, {}>;
    }
  }

  export namespace pets {
    export interface Interface {
      /// List available pets
      list: (
        input: myapi.proto.PetsListRequest,
        headers: myapi.proto.Headers,
      ) => AsyncResult<
        myapi.proto.Paginated<myapi.model.Pet>,
        myapi.proto.PetsListError
      >;
      /// Create a new pet
      create: (
        input: myapi.proto.PetsCreateRequest,
        headers: myapi.proto.Headers,
      ) => AsyncResult<{}, myapi.proto.PetsCreateError>;
      /// Update an existing pet
      update: (
        input: myapi.proto.PetsUpdateRequest,
        headers: myapi.proto.Headers,
      ) => AsyncResult<{}, myapi.proto.PetsUpdateError>;
      /// Remove an existing pet
      remove: (
        input: myapi.proto.PetsRemoveRequest,
        headers: myapi.proto.Headers,
      ) => AsyncResult<{}, myapi.proto.PetsRemoveError>;
      /// Fetch first pet, if any exists
      get_first: (
        input: {},
        headers: myapi.proto.Headers,
      ) => AsyncResult<myapi.model.Pet | null, myapi.proto.UnauthorizedError>;
    }
  }
}
export namespace myapi {
  export namespace model {
    export type Behavior =
      | "Calm"
      | {
          Aggressive: [
            /// aggressiveness level
            number /* f64 */,
            /// some notes
            string,
          ];
        }
      | {
          Other: {
            /// Custom provided description of a behavior
            description: string;
            /// Additional notes
            /// Up to a user to put free text here
            notes?: string;
          };
        };

    export type Kind =
      /// A dog
      | "dog"
      /// A cat
      | "cat";

    export interface Pet {
      /// identity
      name: string;
      /// kind of pet
      kind: myapi.model.Kind;
      /// age of the pet
      age?: number /* u8 */ | null;
      /// behaviors of the pet
      behaviors?: Array<myapi.model.Behavior>;
    }
  }

  export namespace proto {
    export interface Headers {
      authorization: string;
    }

    export interface Paginated<T> {
      /// slice of a collection
      items: Array<T>;
      /// cursor for getting next page
      cursor?: string | null;
    }

    export type PetsCreateError =
      | "Conflict"
      | "NotAuthorized"
      | {
          InvalidIdentity: {
            message: string;
          };
        };

    export type PetsCreateRequest = myapi.model.Pet;

    export type PetsListError = "InvalidCustor" | "Unauthorized";

    export interface PetsListRequest {
      limit?: number /* u8 */ | null;
      cursor?: string | null;
    }

    export type PetsRemoveError = "NotFound" | "NotAuthorized";

    export interface PetsRemoveRequest {
      /// identity
      name: string;
    }

    export type PetsUpdateError = "NotFound" | "NotAuthorized";

    export interface PetsUpdateRequest {
      /// identity
      name: string;
      /// kind of pet, non nullable in the model
      kind?: myapi.model.Kind | null;
      /// age of the pet, nullable in the model
      age?: number /* u8 */ | null | undefined;
      /// behaviors of the pet, nullable in the model
      behaviors?: Array<myapi.model.Behavior> | null | undefined;
    }

    export type UnauthorizedError = null;
  }
}

export namespace reflectapi {
  /// Struct object with no fields
  export interface Empty {}

  /// Error object which is expected to be never returned
  export interface Infallible {}
}

namespace __implementation {
  /* <----- */

  export function __client(base: string | Client): __definition.Interface {
    const client_instance =
      typeof base === "string" ? new ClientInstance(base) : base;
    return {
      impl: {
        health: {
          check: health__check(client_instance),
        },
        pets: {
          list: pets__list(client_instance),
          create: pets__create(client_instance),
          update: pets__update(client_instance),
          remove: pets__remove(client_instance),
          get_first: pets__get_first(client_instance),
        },
      },
    }.impl;
  }

  export function __request<I, H, O, E>(
    client: Client,
    path: string,
    input: I | undefined,
    headers: H | undefined,
  ): AsyncResult<O, E> {
    let hdrs: Record<string, string> = {
      "content-type": "application/json",
    };
    if (headers) {
      for (const [k, v] of Object.entries(headers)) {
        hdrs[k?.toString()] = v?.toString() || "";
      }
    }
    return client
      .request(path, JSON.stringify(input), hdrs)
      .then(([status, response_body]) => {
        if (status < 200 || status >= 300) {
          let parsed_response_body;
          try {
            parsed_response_body = JSON.parse(response_body);
          } catch (e) {
            return new Result<O, Err<E>>({
              err: new Err({ other_err: `[${status}] ${response_body}` }),
            });
          }
          return new Result<O, Err<E>>({
            err: new Err({ application_err: parsed_response_body as E }),
          });
        }

        let parsed_response_body;
        try {
          parsed_response_body = JSON.parse(response_body);
        } catch (e) {
          return new Result<O, Err<E>>({
            err: new Err({
              other_err:
                "internal error: failure to parse response body as json on successful status code: " +
                response_body,
            }),
          });
        }
        return new Result<O, Err<E>>({ ok: parsed_response_body as O });
      })
      .catch((e) => {
        return new Result<O, Err<E>>({ err: new Err({ other_err: e }) });
      });
  }

  class ClientInstance {
    constructor(private base: string) {}

    public request(
      path: string,
      body: string,
      headers: Record<string, string>,
    ): Promise<[number, string]> {
      return (globalThis as any)
        .fetch(`${this.base}${path}`, {
          method: "POST",
          headers: headers,
          body: body,
        })
        .then((response: any) => {
          return response.text().then((text: string) => {
            return [response.status, text];
          });
        });
    }
  }

  /* -----> */

  function health__check(client: Client) {
    return (input: {}, headers: {}) =>
      __request<{}, {}, {}, {}>(client, "/health.check", input, headers);
  }
  function pets__list(client: Client) {
    return (input: myapi.proto.PetsListRequest, headers: myapi.proto.Headers) =>
      __request<
        myapi.proto.PetsListRequest,
        myapi.proto.Headers,
        myapi.proto.Paginated<myapi.model.Pet>,
        myapi.proto.PetsListError
      >(client, "/pets.list", input, headers);
  }
  function pets__create(client: Client) {
    return (
      input: myapi.proto.PetsCreateRequest,
      headers: myapi.proto.Headers,
    ) =>
      __request<
        myapi.proto.PetsCreateRequest,
        myapi.proto.Headers,
        {},
        myapi.proto.PetsCreateError
      >(client, "/pets.create", input, headers);
  }
  function pets__update(client: Client) {
    return (
      input: myapi.proto.PetsUpdateRequest,
      headers: myapi.proto.Headers,
    ) =>
      __request<
        myapi.proto.PetsUpdateRequest,
        myapi.proto.Headers,
        {},
        myapi.proto.PetsUpdateError
      >(client, "/pets.update", input, headers);
  }
  function pets__remove(client: Client) {
    return (
      input: myapi.proto.PetsRemoveRequest,
      headers: myapi.proto.Headers,
    ) =>
      __request<
        myapi.proto.PetsRemoveRequest,
        myapi.proto.Headers,
        {},
        myapi.proto.PetsRemoveError
      >(client, "/pets.remove", input, headers);
  }
  function pets__get_first(client: Client) {
    return (input: {}, headers: myapi.proto.Headers) =>
      __request<
        {},
        myapi.proto.Headers,
        myapi.model.Pet | null,
        myapi.proto.UnauthorizedError
      >(client, "/pets.get-first", input, headers);
  }
}
