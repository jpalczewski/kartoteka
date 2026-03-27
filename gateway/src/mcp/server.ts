import { WorkerEntrypoint } from "cloudflare:workers";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { WebStandardStreamableHTTPServerTransport } from "@modelcontextprotocol/sdk/server/webStandardStreamableHttp.js";
import { registerTools } from "./tools";
import type { Env } from "../types";

interface McpProps {
  userId: string;
}

export class McpApiHandler extends WorkerEntrypoint<Env> {
  async fetch(request: Request): Promise<Response> {
    const { userId } = this.ctx.props as McpProps;

    const server = new McpServer({ name: "kartoteka", version: "1.0.0" });
    registerTools(server, this.env.API_WORKER, this.env.DEV_API_URL, userId);

    // Stateless mode: sessionIdGenerator: undefined — each request is independent.
    // This is correct for Cloudflare Workers where there is no persistent in-memory state.
    const transport = new WebStandardStreamableHTTPServerTransport({
      sessionIdGenerator: undefined,
    });

    await server.connect(transport);
    return transport.handleRequest(request);
  }
}
