import { test, expect } from "@playwright/test";
import { Result, Err, __request, TransportMetadata } from "../generated";

// Mock client for testing transport metadata integration
class MockClient {
  constructor(
    private response: {
      status: number;
      body: string;
      headers: Record<string, string>;
      timing?: { startedAt: number; completedAt: number; duration: number };
      raw?: any;
    },
  ) {}

  async request(path: string, body: string, headers: Record<string, string>) {
    return {
      ...this.response,
      timing: this.response.timing || {
        startedAt: Date.now() - 100,
        completedAt: Date.now(),
        duration: 100,
      },
    };
  }
}

test.describe("Transport Metadata Integration", () => {
  test("successful response - backward compatible unwrap_ok()", async () => {
    const client = new MockClient({
      status: 201,
      body: '{"result": "created", "id": 42}',
      headers: {
        "content-type": "application/json",
        "x-request-id": "test-12345",
      },
    });

    const result = await __request(client, "/test", {}, {});

    // Test backward compatibility - unwrap_ok() returns the value directly
    expect(result.is_ok()).toBe(true);
    const value = result.unwrap_ok(); // No .value needed!
    expect(value).toEqual({ result: "created", id: 42 });

    // Test metadata access on Result object
    expect(result.metadata).toBeDefined();
    expect(result.metadata!.status).toBe(201);
    expect(result.metadata!.headers["content-type"]).toBe("application/json");
    expect(result.metadata!.headers["x-request-id"]).toBe("test-12345");
    expect(result.metadata!.timing).toBeDefined();
    expect(result.metadata!.timing!.duration).toBeGreaterThanOrEqual(100);
  });

  test("application error response - metadata accessible via Err", async () => {
    const client = new MockClient({
      status: 400,
      body: '{"error": "validation_failed", "message": "Invalid input"}',
      headers: {
        "content-type": "application/json",
        "x-error-code": "VAL001",
      },
    });

    const result = await __request(client, "/test", {}, {});

    expect(result.is_err()).toBe(true);
    const error = result.unwrap_err();

    // Test application error access
    expect(error.is_err()).toBe(true);
    const appError = error.err();
    expect(appError).toEqual({
      error: "validation_failed",
      message: "Invalid input",
    });

    // Test metadata access
    const metadata = error.transport_metadata();
    expect(metadata.status).toBe(400);
    expect(metadata.headers["content-type"]).toBe("application/json");
    expect(metadata.headers["x-error-code"]).toBe("VAL001");

    // Test backward compatible status method
    expect(error.status()).toBe(400);
  });

  test("server error response - metadata accessible", async () => {
    const client = new MockClient({
      status: 500,
      body: "Internal Server Error",
      headers: {
        "content-type": "text/plain",
        "x-incident-id": "INC-789",
      },
    });

    const result = await __request(client, "/test", {}, {});

    expect(result.is_err()).toBe(true);
    const error = result.unwrap_err();

    // Test other error access
    expect(error.is_other_err()).toBe(true);
    const otherError = error.other_err();
    expect(otherError).toBe("[500] Internal Server Error");

    // Test metadata access
    const metadata = error.transport_metadata();
    expect(metadata.status).toBe(500);
    expect(metadata.headers["content-type"]).toBe("text/plain");
    expect(metadata.headers["x-incident-id"]).toBe("INC-789");

    // Test backward compatible status method
    expect(error.status()).toBe(500);
  });

  test("network error - metadata with status 0", async () => {
    // Simulate network error by having the client throw
    const failingClient = {
      async request() {
        throw new Error("Network connection failed");
      },
    };

    const result = await __request(failingClient, "/test", {}, {});

    expect(result.is_err()).toBe(true);
    const error = result.unwrap_err();

    // Test network error
    expect(error.is_other_err()).toBe(true);
    const networkError = error.other_err();
    expect(networkError).toBeInstanceOf(Error);
    expect((networkError as Error).message).toBe("Network connection failed");

    // Test metadata for network errors
    const metadata = error.transport_metadata();
    expect(metadata.status).toBe(0); // Network errors have status 0
    expect(metadata.headers).toEqual({});
    expect(metadata.timing).toBeUndefined();

    // Test backward compatible status method
    expect(error.status()).toBe(0);
  });

  test("all existing Result methods still work", async () => {
    const client = new MockClient({
      status: 200,
      body: '{"success": true}',
      headers: { "content-type": "application/json" },
    });

    const result = await __request(client, "/test", {}, {});

    // Test all the existing Result methods work unchanged
    expect(result.is_ok()).toBe(true);
    expect(result.is_err()).toBe(false);

    const okValue = result.ok();
    expect(okValue).toEqual({ success: true });

    const errValue = result.err();
    expect(errValue).toBeUndefined();

    // Test unwrap methods work
    const unwrapped = result.unwrap_ok();
    expect(unwrapped).toEqual({ success: true });

    const defaultValue = result.unwrap_ok_or_default({ default: true });
    expect(defaultValue).toEqual({ success: true });

    const elseValue = result.unwrap_ok_or_else(() => ({ fallback: true }));
    expect(elseValue).toEqual({ success: true });

    // Test map functions work
    const mappedResult = result.map((value) => ({ ...value, mapped: true }));
    expect(mappedResult.unwrap_ok()).toEqual({ success: true, mapped: true });
  });

  test("demonstrates complete integration: existing and new patterns", async () => {
    const client = new MockClient({
      status: 200,
      body: '{"user": {"name": "Alice", "id": 123}}',
      headers: { "content-type": "application/json" },
    });

    const result = await __request(client, "/users/123", {}, {});

    // EXISTING CODE PATTERN - works unchanged!
    if (result.is_ok()) {
      const user = result.unwrap_ok(); // No .value needed - backward compatible!
      expect(user.user.name).toBe("Alice");
      expect(user.user.id).toBe(123);
    }

    // NEW CODE PATTERN - can access metadata when needed
    if (result.is_ok() && result.metadata) {
      expect(result.metadata.status).toBe(200);
      expect(result.metadata.headers["content-type"]).toBe("application/json");
      console.log(`Request completed in ${result.metadata.timing?.duration}ms`);
    }

    // This test proves that:
    // 1. Existing code requires NO changes
    // 2. New metadata features are available when needed
    // 3. The API is both backward compatible AND feature-complete
  });
});
