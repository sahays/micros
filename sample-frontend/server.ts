import { join } from "path";
import { createHmac, createHash, randomBytes } from "node:crypto";

const AUTH_SERVICE_URL = process.env.AUTH_SERVICE_URL || "http://localhost:8080";
const CLIENT_ID = process.env.CLIENT_ID;
const CLIENT_SECRET = process.env.CLIENT_SECRET;
const SIGNING_SECRET = process.env.SIGNING_SECRET;

let appToken: string | null = null;
let appTokenExpiry: number = 0;

async function getAppToken(): Promise<string | null> {
  if (!CLIENT_ID || !CLIENT_SECRET) {
    return null;
  }

  const now = Date.now() / 1000;
  if (appToken && now < appTokenExpiry - 60) {
    return appToken;
  }

  try {
    const res = await fetch(`${AUTH_SERVICE_URL}/auth/app/token`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        client_id: CLIENT_ID,
        client_secret: CLIENT_SECRET,
        grant_type: "client_credentials",
      }),
    });

    if (!res.ok) {
        // Log but don't crash, maybe signing is enough
        console.warn("Failed to get app token:", await res.text());
        return null;
    }

    const data = await res.json() as { access_token: string; expires_in?: number };
    appToken = data.access_token;
    appTokenExpiry = now + (data.expires_in || 3600);
    console.log("Acquired new App Token");
    return appToken;
  } catch (error) {
    console.error("Error fetching app token:", error);
    return null;
  }
}

const server = Bun.serve({
  port: process.env.PORT || 3000,
  async fetch(req) {
    const url = new URL(req.url);
    
    // API Proxy
    if (url.pathname.startsWith("/api")) {
        // Strip /api prefix
        const targetPath = url.pathname.replace(/^\/api/, ""); // /api/auth/x -> /auth/x
        const targetUrl = `${AUTH_SERVICE_URL}${targetPath}${url.search}`;

        // Read body for hashing and forwarding
        const bodyBuffer = await req.arrayBuffer();
        const bodyBufferNode = Buffer.from(bodyBuffer);

        const headers = new Headers(req.headers);
        headers.delete("host");

        // 1. App Token Injection (Optional but implemented)
        const appToken = await getAppToken();
        if (appToken) {
            headers.set("X-App-Token", appToken);
        }

        // 2. Request Signing
        if (CLIENT_ID && SIGNING_SECRET) {
            const timestamp = Math.floor(Date.now() / 1000).toString(); // Seconds
            const nonce = randomBytes(16).toString("hex");
            const method = req.method.toUpperCase();
            // Important: Path should match what the server sees. 
            // If we proxy to /auth/x, the path is /auth/x.
            const path = targetPath; 
            
            const realBodyHash = createHash("sha256").update(bodyBufferNode).digest("hex");

            // Payload: METHOD|PATH|TIMESTAMP|NONCE|BODY_HASH
            const payload = `${method}|${path}|${timestamp}|${nonce}|${realBodyHash}`;
            const signature = createHmac("sha256", SIGNING_SECRET).update(payload).digest("hex");

            headers.set("X-Client-ID", CLIENT_ID);
            headers.set("X-Timestamp", timestamp);
            headers.set("X-Nonce", nonce);
            headers.set("X-Signature", signature);
        }

        try {
            const proxyRes = await fetch(targetUrl, {
                method: req.method,
                headers: headers,
                body: bodyBuffer, // Forward the buffer
                redirect: "manual"
            });
            
            return new Response(proxyRes.body, {
                status: proxyRes.status,
                headers: proxyRes.headers
            });
        } catch (err) {
            console.error("Proxy error:", err);
            return new Response("Proxy Error", { status: 502 });
        }
    }

    // Static File Serving
    let path = url.pathname;
    if (path === "/") path = "/index.html";
    
    const buildDir = "dist"; 
    const filePath = join(process.cwd(), buildDir, path);
    const file = Bun.file(filePath);

    if (await file.exists()) {
      return new Response(file);
    }

    // SPA Fallback
    const indexFile = Bun.file(join(process.cwd(), buildDir, "index.html"));
    if (await indexFile.exists()) {
        return new Response(indexFile);
    }
    
    return new Response("Not Found", { status: 404 });
  },
});

console.log(`Listening on http://localhost:${server.port} ...`);