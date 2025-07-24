import { test, expect } from '@playwright/test';
import { Err, Result, __request, Client, TransportResponse, TransportMetadata } from '../generated';

// Helper function to create TransportResponse for testing
function createMockResponse(status: number, body: string): TransportResponse {
  return {
    status,
    body,
    headers: {},
    timing: {
      startedAt: Date.now(),
      completedAt: Date.now() + 100,
      duration: 100
    },
    raw: undefined
  };
}

// Mock client implementation for testing
class MockClient implements Client {
  constructor(private responses: Map<string, TransportResponse>) {}
  
  async request(path: string, body: string, headers: Record<string, string>): Promise<TransportResponse> {
    const response = this.responses.get(path);
    if (!response) throw new Error(`No mock response for ${path}`);
    return response;
  }
}

test.describe('Err class with HTTP status codes', () => {
  test('application error includes status code', () => {
    const metadata: TransportMetadata = { status: 400, headers: {}, timing: undefined, raw: undefined };
    const err = new Err({ application_err: 'ValidationError', metadata });
    
    expect(err.status()).toBe(400);
    expect(err.is_err()).toBe(true);
    expect(err.is_other_err()).toBe(false);
    expect(err.unwrap()).toBe('ValidationError');
  });

  test('other error includes status code', () => {
    const metadata: TransportMetadata = { status: 500, headers: {}, timing: undefined, raw: undefined };
    const err = new Err({ other_err: 'Network timeout', metadata });
    
    expect(err.status()).toBe(500);
    expect(err.is_err()).toBe(false);
    expect(err.is_other_err()).toBe(true);
    expect(err.other_err()).toBe('Network timeout');
  });

  test('network error has status 0', () => {
    const metadata: TransportMetadata = { status: 0, headers: {}, timing: undefined, raw: undefined };
    const err = new Err({ other_err: 'Connection refused', metadata });
    
    expect(err.status()).toBe(0);
    expect(err.is_other_err()).toBe(true);
  });

  test('error mapping preserves status', () => {
    const metadata: TransportMetadata = { status: 404, headers: {}, timing: undefined, raw: undefined };
    const err = new Err({ application_err: 'NotFound', metadata });
    const mapped = err.map(e => `Mapped: ${e}`);
    
    expect(mapped.status()).toBe(404);
    expect(mapped.unwrap()).toBe('Mapped: NotFound');
  });

  test('toString includes status information', () => {
    const metadata1: TransportMetadata = { status: 400, headers: {}, timing: undefined, raw: undefined };
    const metadata2: TransportMetadata = { status: 500, headers: {}, timing: undefined, raw: undefined };
    const appErr = new Err({ application_err: 'ValidationFailed', metadata: metadata1 });
    const otherErr = new Err({ other_err: 'Server Error', metadata: metadata2 });
    
    expect(appErr.toString()).toContain('ValidationFailed');
    expect(otherErr.toString()).toContain('Server Error');
  });
});

test.describe('__request function with status codes', () => {
  test('success response (200-299)', async () => {
    const client = new MockClient(new Map([
      ['/test', createMockResponse(201, '{"result": "created"}')]
    ]));
    
    const result = await __request(client, '/test', {}, {});
    
    expect(result.is_ok()).toBe(true);
    const apiResult = result.ok()!;
    expect(apiResult.value).toEqual({ result: "created" });
    expect(apiResult.metadata.status).toBe(201);
  });

  test('application error (400-499)', async () => {
    const client = new MockClient(new Map([
      ['/test', createMockResponse(400, '{"error": "validation_failed"}')]
    ]));
    
    const result = await __request(client, '/test', {}, {});
    
    expect(result.is_err()).toBe(true);
    const err = result.err()!;
    expect(err.status()).toBe(400);
    expect(err.is_err()).toBe(true);
    expect(err.unwrap()).toEqual({ error: "validation_failed" });
  });

  test('unauthorized error (401)', async () => {
    const client = new MockClient(new Map([
      ['/test', createMockResponse(401, '{"error": "unauthorized"}')]
    ]));
    
    const result = await __request(client, '/test', {}, {});
    
    expect(result.is_err()).toBe(true);
    const err = result.err()!;
    expect(err.status()).toBe(401);
    expect(err.is_err()).toBe(true);
    expect(err.unwrap()).toEqual({ error: "unauthorized" });
  });

  test('not found error (404)', async () => {
    const client = new MockClient(new Map([
      ['/test', createMockResponse(404, '{"error": "not_found"}')]
    ]));
    
    const result = await __request(client, '/test', {}, {});
    
    expect(result.is_err()).toBe(true);
    const err = result.err()!;
    expect(err.status()).toBe(404);
    expect(err.is_err()).toBe(true);
    expect(err.unwrap()).toEqual({ error: "not_found" });
  });

  test('conflict error (409)', async () => {
    const client = new MockClient(new Map([
      ['/test', createMockResponse(409, '{"error": "conflict"}')]
    ]));
    
    const result = await __request(client, '/test', {}, {});
    
    expect(result.is_err()).toBe(true);
    const err = result.err()!;
    expect(err.status()).toBe(409);
    expect(err.is_err()).toBe(true);
    expect(err.unwrap()).toEqual({ error: "conflict" });
  });

  test('server error (500+)', async () => {
    const client = new MockClient(new Map([
      ['/test', createMockResponse(500, 'Internal Server Error')]
    ]));
    
    const result = await __request(client, '/test', {}, {});
    
    expect(result.is_err()).toBe(true);
    const err = result.err()!;
    expect(err.status()).toBe(500);
    expect(err.is_other_err()).toBe(true);
    expect(err.other_err()).toBe('[500] Internal Server Error');
  });

  test('bad gateway error (502)', async () => {
    const client = new MockClient(new Map([
      ['/test', createMockResponse(502, 'Bad Gateway')]
    ]));
    
    const result = await __request(client, '/test', {}, {});
    
    expect(result.is_err()).toBe(true);
    const err = result.err()!;
    expect(err.status()).toBe(502);
    expect(err.is_other_err()).toBe(true);
    expect(err.other_err()).toBe('[502] Bad Gateway');
  });

  test('invalid JSON in success response falls back to other_err with status', async () => {
    const client = new MockClient(new Map([
      ['/test', createMockResponse(200, 'invalid json')]
    ]));
    
    const result = await __request(client, '/test', {}, {});
    
    expect(result.is_err()).toBe(true);
    const err = result.err()!;
    expect(err.status()).toBe(200);
    expect(err.is_other_err()).toBe(true);
    expect(err.other_err()).toContain('failure to parse response body as json');
  });

  test('invalid JSON in application error falls back to other_err', async () => {
    const client = new MockClient(new Map([
      ['/test', createMockResponse(400, 'invalid json')]
    ]));
    
    const result = await __request(client, '/test', {}, {});
    
    expect(result.is_err()).toBe(true);
    const err = result.err()!;
    expect(err.status()).toBe(400);
    expect(err.is_other_err()).toBe(true);
    expect(err.other_err()).toBe('[400] invalid json');
  });

  test('network error has status 0', async () => {
    const client = new MockClient(new Map()); // No responses - will throw
    
    const result = await __request(client, '/test', {}, {});
    
    expect(result.is_err()).toBe(true);
    const err = result.err()!;
    expect(err.status()).toBe(0);
    expect(err.is_other_err()).toBe(true);
  });
});

test.describe('Backward Compatibility', () => {
  test('existing error handling patterns still work', async () => {
    const client = new MockClient(new Map([
      ['/test', createMockResponse(400, '{"error": "test_error"}')]
    ]));
    
    const result = await __request(client, '/test', {}, {});
    
    // All existing methods should work exactly as before
    expect(result.is_err()).toBe(true);
    const err = result.err()!;
    expect(err.is_err()).toBe(true);
    expect(err.unwrap()).toEqual({ error: "test_error" });
    
    // New status method should also work
    expect(err.status()).toBe(400);
  });

  test('existing client usage patterns unchanged', async () => {
    const client = new MockClient(new Map([
      ['/test', createMockResponse(409, '"Conflict"')]
    ]));
    
    const result = await __request(client, '/test', {}, {});
    
    result.unwrap_ok_or_else((e) => {
      const received_err = e.unwrap(); // Should not throw
      // Existing switch/match patterns should work
      switch (received_err) {
        case 'Conflict': 
          expect(e.status()).toBe(409); // But now we can also check status
          return {};
        case 'NotAuthorized': return {};
        default: return {};
      }
    });
  });

  test('unwrap_or_default methods work with status', () => {
    const metadata: TransportMetadata = { status: 400, headers: {}, timing: undefined, raw: undefined };
    const err = new Err({ application_err: 'TestError', metadata });
    
    expect(err.unwrap_or_default('DefaultError')).toBe('TestError');
    expect(err.status()).toBe(400);
  });

  test('unwrap_or_else methods work with status', () => {
    const metadata: TransportMetadata = { status: 0, headers: {}, timing: undefined, raw: undefined };
    const err = new Err({ other_err: 'NetworkError', metadata });
    
    expect(err.unwrap_or_else(() => 'FallbackError')).toBe('FallbackError');
    expect(err.status()).toBe(0);
  });
});

test.describe('Enhanced Error Handling Use Cases', () => {
  test('can handle different status codes for same error type', async () => {
    const client400 = new MockClient(new Map([
      ['/test', createMockResponse(400, '"ValidationError"')]
    ]));
    const client422 = new MockClient(new Map([
      ['/test', createMockResponse(422, '"ValidationError"')]
    ]));
    
    const result400 = await __request(client400, '/test', {}, {});
    const result422 = await __request(client422, '/test', {}, {});
    
    const err400 = result400.err()!;
    const err422 = result422.err()!;
    
    // Same error content, different status codes
    expect(err400.unwrap()).toBe('ValidationError');
    expect(err422.unwrap()).toBe('ValidationError');
    expect(err400.status()).toBe(400);
    expect(err422.status()).toBe(422);
  });

  test('comprehensive error handling with status-based logic', async () => {
    const scenarios = [
      [400, '"BadRequest"'],
      [401, '"Unauthorized"'], 
      [403, '"Forbidden"'],
      [404, '"NotFound"'],
      [409, '"Conflict"'],
      [422, '"UnprocessableEntity"'],
      [429, '"TooManyRequests"'],
      [500, 'Internal Server Error'],
      [502, 'Bad Gateway'],
      [503, 'Service Unavailable']
    ] as [number, string][];

    for (const [statusCode, response] of scenarios) {
      const client = new MockClient(new Map([['/test', createMockResponse(statusCode, response)]]));
      const result = await __request(client, '/test', {}, {});
      
      expect(result.is_err()).toBe(true);
      const err = result.err()!;
      expect(err.status()).toBe(statusCode);
      
      if (statusCode >= 400 && statusCode < 500) {
        expect(err.is_err()).toBe(true);
      } else if (statusCode >= 500) {
        expect(err.is_other_err()).toBe(true);
      }
    }
  });
});