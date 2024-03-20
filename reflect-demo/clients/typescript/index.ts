/**
 * DO NOT MODIFY THIS FILE MANUALLY
 * This file was generated by reflect-cli
 *
 * Schema name: demo schema
 * demo schema description
 */

/**
 * UTF-8 encoded string
 */
type std.string.String = TODO;

/**
 * Optional / Nullable value type
 */
type std.option.Option<T> = TODO;

/**
 *  Some example doc
 *  test
 */
interface reflect_demo.ExampleRequest {
    inputData: std.string.String;
    input_optional: std.option.Option<std.string.String>;
    }

interface reflect_demo.ExampleResponse {
    /**
     *  some doc
     */
    message: std.string.String;
    }


type reflect_demo.ExampleError =
    Error1 |
    ;

interface reflect_demo.ExampleRequestHeaders {
    name: std.string.String;
    }

/**
 * Error object which is expected to be never returned
 */
type reflect.infallible.Infallible = TODO;

/**
 * Struct object with no fields
 */
type reflect.empty.Empty = TODO;
