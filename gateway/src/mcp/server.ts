import { WorkerEntrypoint } from "cloudflare:workers";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { WebStandardStreamableHTTPServerTransport } from "@modelcontextprotocol/sdk/server/webStandardStreamableHttp.js";
import { registerTools } from "./tools/index";
import type { ApiContext } from "./api";
import type { Env } from "../types";

interface McpProps {
  userId: string;
}

export class McpApiHandler extends WorkerEntrypoint<Env> {
  async fetch(request: Request): Promise<Response> {
    const { userId } = this.ctx.props as McpProps;

    const server = new McpServer({ name: "kartoteka", version: "1.0.0" });
    const api: ApiContext = {
      apiWorker: this.env.API_WORKER,
      devApiUrl: this.env.DEV_API_URL,
      userId,
    };
    registerTools(server, api);

    // Stateless mode: sessionIdGenerator: undefined — each request is independent.
    // This is correct for Cloudflare Workers where there is no persistent in-memory state.
    const transport = new WebStandardStreamableHTTPServerTransport({
      sessionIdGenerator: undefined,
    });

    await server.connect(transport);
    return transport.handleRequest(request);
  }
}
