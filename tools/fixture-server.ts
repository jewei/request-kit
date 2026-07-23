/**
 * Deterministic local HTTP fixture server for interactive development and the
 * manual smoke checklist (docs/smoke.md). Rust integration tests use their own
 * in-process fixture — this one is for humans.
 *
 * Run: bun run fixtures   (default port 4400; PORT env overrides)
 */
const port = Number(process.env.PORT ?? 4400);

function json(body: unknown, init: ResponseInit = {}): Response {
  return new Response(JSON.stringify(body, null, 2), {
    ...init,
    headers: { 'content-type': 'application/json', ...(init.headers ?? {}) },
  });
}

const server = Bun.serve({
  port,
  idleTimeout: 60,
  async fetch(req) {
    const url = new URL(req.url);
    const path = url.pathname;

    if (path === '/json') {
      return json({ hello: 'world', method: req.method, at: new Date().toISOString() });
    }

    if (path === '/echo') {
      const bodyText = await req.text();
      return json({
        method: req.method,
        headers: Object.fromEntries(req.headers.entries()),
        query: Object.fromEntries(url.searchParams.entries()),
        body: bodyText,
      });
    }

    const delay = path.match(/^\/delay\/(\d+)$/);
    if (delay) {
      const secs = Math.min(Number(delay[1]), 300);
      await new Promise((r) => setTimeout(r, secs * 1000));
      return json({ delayed: secs });
    }

    const redirect = path.match(/^\/redirect\/(\d+)$/);
    if (redirect) {
      const n = Number(redirect[1]);
      if (n <= 0) return json({ redirected: true });
      return new Response(null, {
        status: 302,
        headers: { location: `/redirect/${n - 1}` },
      });
    }

    if (path === '/dup-headers') {
      const headers = new Headers();
      headers.append('set-cookie', 'a=1');
      headers.append('set-cookie', 'b=2');
      headers.set('content-type', 'application/json');
      return new Response(JSON.stringify({ dup: true }), { headers });
    }

    if (path === '/gzip') {
      const payload = JSON.stringify({ compressed: true, filler: 'x'.repeat(2048) });
      const gz = Bun.gzipSync(Buffer.from(payload));
      return new Response(gz, {
        headers: {
          'content-type': 'application/json',
          'content-encoding': 'gzip',
        },
      });
    }

    if (path === '/binary') {
      const bytes = new Uint8Array(256).map((_, i) => i);
      return new Response(bytes, {
        headers: { 'content-type': 'application/octet-stream' },
      });
    }

    const size = path.match(/^\/size\/(\d+)$/);
    if (size) {
      const kb = Math.min(Number(size[1]), 200_000);
      return new Response('a'.repeat(kb * 1024), {
        headers: { 'content-type': 'text/plain' },
      });
    }

    if (path === '/close') {
      // Abrupt connection close: hijack by returning a stream that errors.
      return new Response(
        new ReadableStream({
          start(controller) {
            controller.enqueue(new TextEncoder().encode('partial'));
            controller.error(new Error('abrupt close'));
          },
        }),
        { headers: { 'content-type': 'text/plain' } },
      );
    }

    if (path === '/status') {
      const code = Number(url.searchParams.get('code') ?? 200);
      return json({ requested: code }, { status: code });
    }

    return json({ error: 'unknown fixture', path }, { status: 404 });
  },
});

console.log(`fixture server listening on http://localhost:${server.port}`);
console.log(
  'endpoints: /json /echo /delay/:s /redirect/:n /dup-headers /gzip /binary /size/:kb /close /status?code=NNN',
);
