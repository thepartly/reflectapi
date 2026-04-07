export interface RequestOptions {
  signal?: AbortSignal;
}

export interface Client {
  request(
    path: string,
    body: string,
    headers: Record<string, string>,
    options?: RequestOptions,
  ): Promise<Response>;
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
  options?: RequestOptions,
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
    .request(path, JSON.stringify(input), hdrs, options)
    .then(async (response) => {
      const response_body = await response.text();
      if (response.status >= 200 && response.status < 300) {
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
      } else if (response.status >= 500) {
        return new Result<O, Err<E>>({
          err: new Err({ other_err: `[${response.status}] ${response_body}` }),
        })
      } else {
        try {
          return new Result<O, Err<E>>({
            err: new Err({ application_err: JSON.parse(response_body) as E }),
          });
        } catch (e) {
          return new Result<O, Err<E>>({
            err: new Err({ other_err: `[${response.status}] ${response_body}` }),
          });
        }
      }
    })
    .catch((e) => {
      return new Result<O, Err<E>>({ err: new Err({ other_err: e }) });
    });
}

export async function __stream_request<I, H, O, E>(
  client: Client,
  path: string,
  input: I | undefined,
  headers: H | undefined,
  options?: RequestOptions,
): Promise<Result<AsyncIterable<O>, Err<E>>> {
  let hdrs: Record<string, string> = {
    "content-type": "application/json",
    "accept": "text/event-stream",
  };
  if (headers) {
    for (const [k, v] of Object.entries(headers)) {
      hdrs[k?.toString()] = v?.toString() || "";
    }
  }
  try {
    const response = await client.request(path, JSON.stringify(input), hdrs, options);
    if (response.status >= 200 && response.status < 300) {
      const stream = __sse_to_async_iterable<O>(response, options);
      return new Result<AsyncIterable<O>, Err<E>>({ ok: stream });
    } else if (response.status >= 500) {
      const body = await response.text();
      return new Result<AsyncIterable<O>, Err<E>>({
        err: new Err({ other_err: `[${response.status}] ${body}` }),
      });
    } else {
      const body = await response.text();
      try {
        return new Result<AsyncIterable<O>, Err<E>>({
          err: new Err({ application_err: JSON.parse(body) as E }),
        });
      } catch (e) {
        return new Result<AsyncIterable<O>, Err<E>>({
          err: new Err({ other_err: `[${response.status}] ${body}` }),
        });
      }
    }
  } catch (e) {
    return new Result<AsyncIterable<O>, Err<E>>({
      err: new Err({ other_err: e }),
    });
  }
}

export async function __stream_request_fallible<I, H, O, IE, E>(
  client: Client,
  path: string,
  input: I | undefined,
  headers: H | undefined,
  options?: RequestOptions,
): Promise<Result<AsyncIterable<Result<O, IE>>, Err<E>>> {
  let hdrs: Record<string, string> = {
    "content-type": "application/json",
    "accept": "text/event-stream",
  };
  if (headers) {
    for (const [k, v] of Object.entries(headers)) {
      hdrs[k?.toString()] = v?.toString() || "";
    }
  }
  try {
    const response = await client.request(path, JSON.stringify(input), hdrs, options);
    if (response.status >= 200 && response.status < 300) {
      const stream = __sse_to_async_iterable_fallible<O, IE>(response, options);
      return new Result<AsyncIterable<Result<O, IE>>, Err<E>>({ ok: stream });
    } else if (response.status >= 500) {
      const body = await response.text();
      return new Result<AsyncIterable<Result<O, IE>>, Err<E>>({
        err: new Err({ other_err: `[${response.status}] ${body}` }),
      });
    } else {
      const body = await response.text();
      try {
        return new Result<AsyncIterable<Result<O, IE>>, Err<E>>({
          err: new Err({ application_err: JSON.parse(body) as E }),
        });
      } catch (e) {
        return new Result<AsyncIterable<Result<O, IE>>, Err<E>>({
          err: new Err({ other_err: `[${response.status}] ${body}` }),
        });
      }
    }
  } catch (e) {
    return new Result<AsyncIterable<Result<O, IE>>, Err<E>>({
      err: new Err({ other_err: e }),
    });
  }
}

async function* __sse_to_async_iterable<O>(
  response: Response,
  options?: RequestOptions,
): AsyncIterable<O> {
  const body = response.body;
  if (!body) return;
  const reader = body.pipeThrough(new TextDecoderStream()).pipeThrough(new __EventSourceParserStream()).getReader();
  try {
    while (true) {
      if (options?.signal?.aborted) break;
      const { done, value } = await reader.read();
      if (done) break;
      const parsed = JSON.parse(value.data);
      yield ('ok' in parsed ? parsed.ok : parsed) as O;
    }
  } finally {
    reader.cancel().catch(() => {});
  }
}

async function* __sse_to_async_iterable_fallible<O, IE>(
  response: Response,
  options?: RequestOptions,
): AsyncIterable<Result<O, IE>> {
  const body = response.body;
  if (!body) return;
  const reader = body.pipeThrough(new TextDecoderStream()).pipeThrough(new __EventSourceParserStream()).getReader();
  try {
    while (true) {
      if (options?.signal?.aborted) break;
      const { done, value } = await reader.read();
      if (done) break;
      const parsed = JSON.parse(value.data);
      if ("ok" in parsed) {
        yield new Result<O, IE>({ ok: parsed.ok as O });
      } else if ("err" in parsed) {
        yield new Result<O, IE>({ err: parsed.err as IE });
      }
    }
  } finally {
    reader.cancel().catch(() => {});
  }
}

class ClientInstance {
  constructor(private base: string) { }

  public request(
    path: string,
    body: string,
    headers: Record<string, string>,
    options?: RequestOptions,
  ): Promise<Response> {
    return (globalThis as any).fetch(`${this.base}${path}`, {
      method: "POST",
      headers: headers,
      body: body,
      signal: options?.signal,
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
 * Ergonomically and exhaustively handle all possible cases of a discriminated union 
 * in the externally tagged representation (https://serde.rs/enum-representations.html).
 * 
 * @example
 * ```typescript
 * type Status = 'loading' | { success: { data: string } } | { error: { message: string } };
 * 
 * function handleStatus(status: Status) {
 *   return match(status, {
 *     loading: () => 'Loading...',
 *     success: ({ data }) => `Success: ${data}`,
 *     error: ({ message }) => `Error: ${message}`
 *   });
 * }
 * 
 * // With default handler for non-exhaustive matching
 * function handleStatusWithDefault(status: Status) {
 *   return match(
 *     status,
 *     { loading: () => 'Loading...' },
 *     () => 'Unknown status'
 *   );
 * }
 * ```
 */
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

// Vendored from eventsource-parser v3.0.6 (MIT License)
// Copyright (c) 2025 Espen Hovlandsdal <espen@hovlandsdal.com>
// https://github.com/rexxars/eventsource-parser

interface __EventSourceMessage {
  event?: string | undefined
  id?: string | undefined
  data: string
}

interface __EventSourceParser {
  feed(chunk: string): void
  reset(options?: { consume?: boolean }): void
}

function __createParser(callbacks: {
  onEvent?: (event: __EventSourceMessage) => void
  onRetry?: (retry: number) => void
  onComment?: (comment: string) => void
  onError?: (error: Error) => void
}): __EventSourceParser {
  const { onEvent = () => { }, onError = () => { }, onRetry = () => { }, onComment } = callbacks
  let incompleteLine = ''
  let isFirstChunk = true
  let id: string | undefined
  let data = ''
  let eventType = ''

  function feed(newChunk: string) {
    const chunk = isFirstChunk ? newChunk.replace(/^\xEF\xBB\xBF/, '') : newChunk
    const [complete, incomplete] = __splitLines(`${incompleteLine}${chunk}`)
    for (const line of complete) {
      parseLine(line)
    }
    incompleteLine = incomplete
    isFirstChunk = false
  }

  function parseLine(line: string) {
    if (line === '') {
      dispatchEvent()
      return
    }
    if (line.startsWith(':')) {
      if (onComment) onComment(line.slice(line.startsWith(': ') ? 2 : 1))
      return
    }
    const fieldSeparatorIndex = line.indexOf(':')
    if (fieldSeparatorIndex !== -1) {
      const field = line.slice(0, fieldSeparatorIndex)
      const offset = line[fieldSeparatorIndex + 1] === ' ' ? 2 : 1
      const value = line.slice(fieldSeparatorIndex + offset)
      processField(field, value)
      return
    }
    processField(line, '')
  }

  function processField(field: string, value: string) {
    switch (field) {
      case 'event': eventType = value; break
      case 'data': data = `${data}${value}\n`; break
      case 'id': id = value.includes('\0') ? undefined : value; break
      case 'retry':
        if (/^\d+$/.test(value)) onRetry(parseInt(value, 10))
        break
    }
  }

  function dispatchEvent() {
    if (data.length > 0) {
      onEvent({
        id,
        event: eventType || undefined,
        data: data.endsWith('\n') ? data.slice(0, -1) : data,
      })
    }
    id = undefined
    data = ''
    eventType = ''
  }

  function reset(options: { consume?: boolean } = {}) {
    if (incompleteLine && options.consume) parseLine(incompleteLine)
    isFirstChunk = true
    id = undefined
    data = ''
    eventType = ''
    incompleteLine = ''
  }

  return { feed, reset }
}

function __splitLines(chunk: string): [Array<string>, string] {
  const lines: Array<string> = []
  let incompleteLine = ''
  let searchIndex = 0
  while (searchIndex < chunk.length) {
    const crIndex = chunk.indexOf('\r', searchIndex)
    const lfIndex = chunk.indexOf('\n', searchIndex)
    let lineEnd = -1
    if (crIndex !== -1 && lfIndex !== -1) {
      lineEnd = Math.min(crIndex, lfIndex)
    } else if (crIndex !== -1) {
      if (crIndex === chunk.length - 1) lineEnd = -1
      else lineEnd = crIndex
    } else if (lfIndex !== -1) {
      lineEnd = lfIndex
    }
    if (lineEnd === -1) {
      incompleteLine = chunk.slice(searchIndex)
      break
    } else {
      lines.push(chunk.slice(searchIndex, lineEnd))
      searchIndex = lineEnd + 1
      if (chunk[searchIndex - 1] === '\r' && chunk[searchIndex] === '\n') searchIndex++
    }
  }
  return [lines, incompleteLine]
}

class __EventSourceParserStream extends TransformStream<string, __EventSourceMessage> {
  constructor() {
    let parser!: __EventSourceParser
    super({
      start(controller) {
        parser = __createParser({
          onEvent: (event) => { controller.enqueue(event) },
        })
      },
      transform(chunk) { parser.feed(chunk) },
    })
  }
}
