// Simple hello world edge function for testing
Deno.serve(async (req: Request) => {
  const data = {
    message: "Hello from Edge Functions!",
  };

  return new Response(JSON.stringify(data), {
    headers: { "Content-Type": "application/json" },
  });
});
