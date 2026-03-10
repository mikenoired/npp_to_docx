import { handleApi } from "./api";
import { embeddedAssets } from "./embedded-assets";

function json(data: unknown, status = 200): Response {
  return new Response(JSON.stringify(data, null, 2), {
    status,
    headers: {
      "content-type": "application/json; charset=utf-8",
    },
  });
}

function getAsset(pathname: string) {
  return embeddedAssets[pathname] ?? embeddedAssets["/index.html"];
}

function serveAsset(pathname: string): Response {
  const asset = getAsset(pathname);
  const body = asset.encoding === "base64" ? Buffer.from(asset.body, "base64") : asset.body;
  return new Response(body, {
    headers: {
      "content-type": asset.contentType,
    },
  });
}

const port = Number.parseInt(process.env.PORT ?? "3000", 10) || 3000;

const server = Bun.serve({
  port,
  async fetch(request: Request) {
    const url = new URL(request.url);
    if (url.pathname.startsWith("/api/")) {
      try {
        return await handleApi(request);
      } catch (error) {
        return json({ error: error instanceof Error ? error.message : String(error) }, 500);
      }
    }

    return serveAsset(url.pathname === "/" ? "/index.html" : url.pathname);
  },
});

console.log(`Web server listening on http://127.0.0.1:${server.port}`);
