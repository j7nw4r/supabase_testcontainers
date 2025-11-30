// Main service entry point for edge-runtime testing
console.log("main function started");

Deno.serve(async (req: Request) => {
  const url = new URL(req.url);
  const { pathname } = url;

  // Handle health checks
  if (pathname === "/_internal/health" || pathname === "/health") {
    return new Response(
      JSON.stringify({ message: "ok" }),
      {
        status: 200,
        headers: { "Content-Type": "application/json" },
      },
    );
  }

  // Route to hello function
  if (pathname === "/hello" || pathname.startsWith("/hello/")) {
    return new Response(
      JSON.stringify({ message: "Hello from Edge Functions!" }),
      {
        status: 200,
        headers: { "Content-Type": "application/json" },
      },
    );
  }

  // Default 404 for unknown paths
  return new Response(
    JSON.stringify({ error: "Not found", path: pathname }),
    {
      status: 404,
      headers: { "Content-Type": "application/json" },
    },
  );
});
