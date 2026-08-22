const encoder = new TextEncoder();

function b64url(input: Uint8Array | ArrayBuffer): string {
  const bytes = input instanceof Uint8Array ? input : new Uint8Array(input);
  let str = "";
  for (const byte of bytes) str += String.fromCharCode(byte);
  return btoa(str).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

async function signJwt(
  payload: Record<string, unknown>,
  secret: string,
): Promise<string> {
  const header = b64url(encoder.encode(JSON.stringify({ alg: "HS256", typ: "JWT" })));
  const claims = b64url(encoder.encode(JSON.stringify(payload)));
  const data = `${header}.${claims}`;
  const key = await crypto.subtle.importKey(
    "raw",
    encoder.encode(secret),
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"],
  );
  const signature = await crypto.subtle.sign("HMAC", key, encoder.encode(data));
  return `${data}.${b64url(signature)}`;
}

Deno.serve(async (req) => {
  if (req.method !== "POST") {
    return new Response("Method not allowed", { status: 405 });
  }

  const jwtSecret = Deno.env.get("DATABASE_JWT_SECRET");
  if (!jwtSecret) {
    return new Response("Server not configured", { status: 500 });
  }

  let body: { access_token?: unknown };
  try {
    body = await req.json();
  } catch {
    return new Response("Invalid JSON body", { status: 400 });
  }

  const accessToken = body.access_token;
  if (
    typeof accessToken !== "string" ||
    accessToken.length === 0 ||
    accessToken.length > 4096
  ) {
    return new Response("Missing or invalid access_token", { status: 400 });
  }

  let profileResponse: Response;
  try {
    profileResponse = await fetch(
      "https://api.minecraftservices.com/minecraft/profile",
      { headers: { Authorization: `Bearer ${accessToken}` } },
    );
  } catch {
    return new Response("Mojang API unreachable", { status: 502 });
  }

  if (!profileResponse.ok) {
    return new Response("Invalid Minecraft token", { status: 401 });
  }

  const profile = await profileResponse.json();
  const mcUuid = profile?.id;
  if (typeof mcUuid !== "string" || mcUuid.length === 0) {
    return new Response("Profile unavailable", { status: 401 });
  }

  const issuedAt = Math.floor(Date.now() / 1000);
  const expiresIn = 3600;

  const token = await signJwt(
    {
      sub: mcUuid,
      role: "authenticated",
      aud: "authenticated",
      iat: issuedAt,
      exp: issuedAt + expiresIn,
    },
    jwtSecret,
  );

  return Response.json({ token, expires_in: expiresIn });
});