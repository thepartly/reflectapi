export interface Client {
  request(
    path: string,
    body: string,
    headers: Record<string, string>,
  ): Promise<[number, string]>;
}

export type NullToEmptyObject<T> = T extends null ? {} : T;

export type AsyncResult<T, E> = Promise<Result<T, Err<E>>>;

export type FixedSizeArray<T, N extends number> = Array<T> & { length: N };

export class Result<T, E> {
  constructor(private value: { ok: T } | { err: E }) { }

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
      `called \`unwrap_ok\` on an \`err\` value: ${JSON.stringify(this.value.err)}`,
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
      return `Ok { ok: ${JSON.stringify(this.value.ok)} }`;
    } else {
      return `Err { err: ${JSON.stringify(this.value.err)} }`;
    }
  }
}

export class Err<E> {
  constructor(private value: { application_err: E } | { other_err: any }) { }

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
      return `Application Error: ${JSON.stringify(this.value.application_err)}`;
    } else {
      return `Other Error: ${JSON.stringify(this.value.other_err)}`;
    }
  }
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
      if (status >= 200 && status < 300) {
        try {
          return new Result<O, Err<E>>({ ok: JSON.parse(response_body) as O });
        } catch (e) {
          return new Result<O, Err<E>>({
            err: new Err({
              other_err:
                "internal error: failure to parse response body as json on successful status code: " +
                response_body,
            }),
          });
        }
      } else if (status >= 500) {
        return new Result<O, Err<E>>({
          err: new Err({ other_err: `[${status}] ${response_body}` }),
        })
      } else {
        try {
          return new Result<O, Err<E>>({
            err: new Err({ application_err: JSON.parse(response_body) as E }),
          });
        } catch (e) {
          return new Result<O, Err<E>>({
            err: new Err({ other_err: `[${status}] ${response_body}` }),
          });
        }
      }
    })
    .catch((e) => {
      return new Result<O, Err<E>>({ err: new Err({ other_err: e }) });
    });
}

class ClientInstance {
  constructor(private base: string) { }

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

type UnionToIntersection<U> = (U extends any ? (k: U) => unknown : never) extends (
  k: infer I
) => void
  ? I
  : never;

// How it works:
// type Step0 =
// 	{ a: { x: string } }
// 	| { b: number }
// 	| 'c'

// First transformation:
// Turn the `string` variant into an object variant so each variant of the union is now uniform.
// type AfterStep1 =
// 	{ a: { x: string } }
// 	| { b: number }
// 	| { c: {} }

type Step1<T> = T extends object ? T : T extends string ? { [K in T]: unknown } : never;

// Second transformation:
// We want to merge the unions into a single object type.
// This is implemented by turning the union into an intersection.
// type AfterStep2 = {
// 	a: { x: string }
// 	b: number
// 	c: {}
// }

type Step2<T> = UnionToIntersection<T>;

// Final transformation:
// Turn each value type into a function that takes the value type as an argument and returns the result type.
// type AfterStep3<R> = {
// 	a: (arg: { x: string }) => R
// 	b: (arg: number) => R
// 	c: (arg: {}) => R
// }

type Step3<T, R> = { [K in keyof T]: (arg: T[K]) => R };

type Cases<T, R> = Step3<Step2<Step1<T>>, R>;

type CasesNonExhaustive<T, R> = Partial<Cases<T, R>>;

/**
 * Ergonomically and exhaustively handle all possible cases of a discriminated union in the externally tagged representation (https://serde.rs/enum-representations.html).
 * See tests for examples.
 * Talk to @andy if you have issues or question or just change it however you wish :)
 * */
export function match<T extends object | string, R>(value: T, cases: Cases<T, R>): R;
export function match<T extends object | string, R>(
  value: T,
  cases: CasesNonExhaustive<T, R>,
  otherwise: () => R
): R;
export function match<T extends object | string, R>(
  value: T,
  cases: Cases<T, R> | CasesNonExhaustive<T, R>,
  otherwise?: () => R
): R {
  const branches = cases as Record<string, (arg: any) => R>;
  switch (typeof value) {
    case 'string':
      if (value in branches) {
        return branches[value]({});
      }

      if (!otherwise) {
        throw new Error('otherwise must exist for non-exhaustive match');
      }
      return otherwise();

    case 'object': {
      if (Object.keys(value).length !== 1) {
        throw new Error(
          'Expected object with exactly one key, see serde documentation for externally tagged enums above'
        );
      }

      const [k, v] = Object.entries(value)[0];
      if (k in branches) {
        return branches[k](v);
      }
      if (!otherwise) {
        throw new Error('otherwise must exist for non-exhaustive match');
      }
      return otherwise();
    }

    default:
      throw new Error('unreachable');
  }
}
