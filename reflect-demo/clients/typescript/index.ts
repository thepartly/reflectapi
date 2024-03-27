// DO NOT MODIFY THIS FILE MANUALLY
// This file was generated by reflect-cli
//
// Schema name: Demo application
// This is a demo application

export function client(base_url: string): __client.Interface {
    return __client_impl(base_url)
}

export class Result<T, E> {
    constructor(private value: { ok: T } | { err: E }) {}

    public ok(): T | undefined {
        if ('ok' in this.value) {
            return this.value.ok;
        }
    }
    public err(): E | undefined {
        if ('err' in this.value) {
            return this.value.err;
        }
    }

    public is_ok(): boolean {
        return 'ok' in this.value;
    }
    public is_err(): boolean {
        return 'err' in this.value;
    }

    public map<U>(f: (r: T) => U): Result<U, E> {
        if ('ok' in this.value) {
            return new Result({ ok: f(this.value.ok) });
        } else {
            return new Result({ err: this.value.err });
        }
    }
    public map_err<U>(f: (r: E) => U): Result<T, U> {
        if ('err' in this.value) {
            return new Result({ err: f(this.value.err) });
        } else {
            return new Result({ ok: this.value.ok });
        }
    }

    public unwrap_ok(): T {
        if ('ok' in this.value) {
            return this.value.ok;
        }
        throw new Error('called `unwrap_ok` on an `err` value: ' + this.value.toString());
    }
    public unwrap_err(): E {
        if ('err' in this.value) {
            return this.value.err;
        }
        throw new Error('called `unwrap_err` on an `ok` value');
    }

    public unwrap_ok_or_default(default_: T): T {
        if ('ok' in this.value) {
            return this.value.ok;
        }
        return default_;
    }
    public unwrap_err_or_default(default_: E): E {
        if ('err' in this.value) {
            return this.value.err;
        }
        return default_;
    }

    public unwrap_ok_or_else(f: () => T): T {
        if ('ok' in this.value) {
            return this.value.ok;
        }
        return f();
    }
    public unwrap_err_or_else(f: () => E): E {
        if ('err' in this.value) {
            return this.value.err;
        }
        return f();
    }

    public toString(): string {
        if ('ok' in this.value) {
            return `Ok { ok: ${this.value.ok} }`;
        } else {
            return `Err { err: ${this.value.err} }`;
        }
    }
}

export class Err<E> {
    constructor(private value: { server_err: E } | { network_err: any }) { }

    public err(): E | undefined {
        if ('server_err' in this.value) {
            return this.value.server_err;
        }
    }
    public network_err(): any | undefined {
        if ('network_err' in this.value) {
            return this.value.network_err;
        }
    }

    public is_err(): boolean {
        return 'server_err' in this.value;
    }
    public is_network_err(): boolean {
        return 'network_err' in this.value;
    }

    public map<U>(f: (r: E) => U): Err<U> {
        if ('server_err' in this.value) {
            return new Err({ server_err: f(this.value.server_err) });
        } else {
            return new Err({ network_err: this.value.network_err });
        }
    }
    public unwrap(): E {
        if ('server_err' in this.value) {
            return this.value.server_err;
        } else {
            throw this.value;
        }
    }
    public unwrap_or_default(default_: E): E {
        if ('server_err' in this.value) {
            return this.value.server_err;
        }
        return default_;
    }
    public unwrap_or_else(f: () => E): E {
        if ('server_err' in this.value) {
            return this.value.server_err;
        }
        return f();
    }

    public toString(): string {
        if ('server_err' in this.value) {
            return `Err { server_err: ${this.value.server_err} }`;
        } else {
            return `Err { network_err: ${this.value.network_err} }`;
        }
    }
}

function __client_impl(base_url: string): __client.Interface {
    return { impl: {
        pets: {
            list: pets__list,
            create: pets__create,
            update: pets__update,
            remove: pets__remove,
        },
        health: {
            check: health__check,
        },
    }, }.impl
}


export namespace __client {

export interface Interface {
    health: health.Interface,
    pets: pets.Interface,
    }


export namespace health {

export interface Interface {
    /// Check the health of the service
    check: () => Promise<Result<void, Err<void>>>,
    }

}

export namespace pets {

export interface Interface {
    /// List available pets
    list: (input: myapi.proto.PetsListRequest, headers: myapi.proto.Headers) => Promise<Result<myapi.proto.Paginated<myapi.model.Pet>, Err<myapi.proto.PetsListError>>>,
    /// Create a new pet
    create: (input: myapi.proto.PetsCreateRequest, headers: myapi.proto.Headers) => Promise<Result<void, Err<myapi.proto.PetsCreateError>>>,
    /// Update an existing pet
    update: (input: myapi.proto.PetsUpdateRequest, headers: myapi.proto.Headers) => Promise<Result<void, Err<myapi.proto.PetsUpdateError>>>,
    /// Remove an existing pet
    remove: (input: myapi.proto.PetsRemoveRequest, headers: myapi.proto.Headers) => Promise<Result<void, Err<myapi.proto.PetsRemoveError>>>,
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
            string
        ]
    }
    | {
        Other: {
            /// Custom provided description of a behavior
            description: string,
            /// Additional notes
            /// Up to a user to put free text here
            notes: string
        }
    }
    ;


export type Kind =
    /// A dog
    | "dog"
    /// A cat
    | "cat"
    ;


export interface Pet {
    /// identity
    name: string,
    /// kind of pet
    kind: myapi.model.Kind,
    /// age of the pet
    age: number /* u8 */ | null,
    /// behaviors of the pet
    behaviors: Array<myapi.model.Behavior>,
    }

}

export namespace proto {

export interface Headers {
    authorization: string,
    }


export interface Paginated<T> {
    /// slice of a collection
    items: Array<T>,
    /// cursor for getting next page
    cursor: string | null,
    }


export type PetsCreateError =
    | "Conflict"
    | "NotAuthorized"
    | {
        InvalidIdentity: {
            message: string
        }
    }
    ;


export type PetsCreateRequest = myapi.model.Pet;


export type PetsListError =
    | "InvalidCustor"
    | "Unauthorized"
    ;


export interface PetsListRequest {
    limit: number /* u8 */,
    cursor: string | null,
    }


export type PetsRemoveError =
    | "NotFound"
    | "NotAuthorized"
    ;


export interface PetsRemoveRequest {
    /// identity
    name: string,
    }


export type PetsUpdateError =
    | "NotFound"
    | "NotAuthorized"
    ;


export interface PetsUpdateRequest {
    /// identity
    name: string,
    /// kind of pet, non nullable in the model
    kind: myapi.model.Kind | null,
    /// age of the pet, nullable in the model
    age: number /* u8 */ | null | undefined,
    /// behaviors of the pet, nullable in the model
    behaviors: Array<myapi.model.Behavior> | null | undefined,
    }

}
}

export namespace reflect {

/// Struct object with no fields
export interface Empty {
    }


/// Error object which is expected to be never returned
export interface Infallible {
    }

}












