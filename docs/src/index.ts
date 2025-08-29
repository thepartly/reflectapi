/**
 * Cloudflare Worker for serving `reflectapi` documentation
 * Uses Workers Static Assets feature to serve mdBook output
 */

export interface Env {
  ASSETS: Fetcher;
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    // Get the path from the request
    const url = new URL(request.url);
    let pathname = url.pathname;

    // Handle root path - serve index.html
    if (pathname === '/') {
      pathname = '/index.html';
    }

    // Handle directory paths - try to serve index.html
    if (pathname.endsWith('/')) {
      pathname += 'index.html';
    }

    try {
      // Try to serve the static asset
      const response = await env.ASSETS.fetch(new URL(pathname, request.url));
      
      if (response.status === 404) {
        // For SPA-style routing, try serving index.html for HTML requests
        if (request.headers.get('accept')?.includes('text/html')) {
          return await env.ASSETS.fetch(new URL('/index.html', request.url));
        }
        
        // Otherwise return the 404
        return new Response('Not Found', { status: 404 });
      }

      // Clone the response and add security headers
      const newResponse = new Response(response.body, response);
      
      // Add security headers
      newResponse.headers.set('X-Frame-Options', 'DENY');
      newResponse.headers.set('X-Content-Type-Options', 'nosniff');
      newResponse.headers.set('Referrer-Policy', 'strict-origin-when-cross-origin');
      
      // Cache static assets for 1 hour, HTML for 5 minutes
      if (pathname.endsWith('.html')) {
        newResponse.headers.set('Cache-Control', 'public, max-age=300');
      } else {
        newResponse.headers.set('Cache-Control', 'public, max-age=3600');
      }

      return newResponse;
    } catch (error) {
      return new Response('Internal Server Error', { status: 500 });
    }
  },
};