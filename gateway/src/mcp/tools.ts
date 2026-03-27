import { z } from "zod";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";

async function apiCall(
  apiWorker: Fetcher | undefined,
  devApiUrl: string | undefined,
  method: string,
  path: string,
  userId: string,
  body?: unknown
): Promise<Response> {
  const headers = new Headers({
    "Content-Type": "application/json",
    "X-User-Id": userId,
  });
  const init: RequestInit = { method, headers };
  if (body !== undefined) init.body = JSON.stringify(body);

  if (devApiUrl) return fetch(`${devApiUrl}${path}`, init);
  return apiWorker!.fetch(new Request(`https://api-worker${path}`, init));
}

export function registerTools(
  server: McpServer,
  apiWorker: Fetcher | undefined,
  devApiUrl: string | undefined,
  userId: string
): void {
  server.registerTool(
    "list_lists",
    {
      description: "List all lists (todo lists) for the current user",
      inputSchema: {},
    },
    async () => {
      const res = await apiCall(apiWorker, devApiUrl, "GET", "/api/lists", userId);
      const data = await res.json();
      return { content: [{ type: "text", text: JSON.stringify(data, null, 2) }] };
    }
  );

  server.registerTool(
    "get_list_items",
    {
      description: "Get all items in a specific list",
      inputSchema: {
        list_id: z.string().describe("The ID of the list to fetch items from"),
      },
    },
    async ({ list_id }) => {
      const res = await apiCall(
        apiWorker,
        devApiUrl,
        "GET",
        `/api/lists/${list_id}/items`,
        userId
      );
      const data = await res.json();
      return { content: [{ type: "text", text: JSON.stringify(data, null, 2) }] };
    }
  );

  server.registerTool(
    "create_list",
    {
      description: "Create a new list",
      inputSchema: {
        name: z.string().describe("The name for the new list"),
      },
    },
    async ({ name }) => {
      const res = await apiCall(apiWorker, devApiUrl, "POST", "/api/lists", userId, {
        name,
      });
      const data = await res.json();
      return { content: [{ type: "text", text: JSON.stringify(data, null, 2) }] };
    }
  );

  server.registerTool(
    "add_item",
    {
      description: "Add a new item to a list",
      inputSchema: {
        list_id: z.string().describe("The ID of the list to add the item to"),
        title: z.string().describe("The title/text of the item"),
      },
    },
    async ({ list_id, title }) => {
      const res = await apiCall(
        apiWorker,
        devApiUrl,
        "POST",
        `/api/lists/${list_id}/items`,
        userId,
        { title }
      );
      const data = await res.json();
      return { content: [{ type: "text", text: JSON.stringify(data, null, 2) }] };
    }
  );

  server.registerTool(
    "toggle_item",
    {
      description: "Toggle the completed state of an item",
      inputSchema: {
        list_id: z.string().describe("The ID of the list containing the item"),
        item_id: z.string().describe("The ID of the item to toggle"),
        completed: z.boolean().describe("The new completed state for the item"),
      },
    },
    async ({ list_id, item_id, completed }) => {
      const res = await apiCall(
        apiWorker,
        devApiUrl,
        "PUT",
        `/api/lists/${list_id}/items/${item_id}`,
        userId,
        { completed }
      );
      const data = await res.json();
      return { content: [{ type: "text", text: JSON.stringify(data, null, 2) }] };
    }
  );
}
