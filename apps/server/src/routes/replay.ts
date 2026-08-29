import type { FastifyInstance } from "fastify";

export async function replayRoutes(app: FastifyInstance) {
  app.post("/replay", async (request, reply) => {
    return { status: "queued" };
  });

  app.get("/replay/:jobId", async (request, reply) => {
    return { status: "pending" };
  });
}
