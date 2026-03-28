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
      if (!res.ok) {
        const text = await res.text();
        return { isError: true, content: [{ type: "text" as const, text: `API error ${res.status}: ${text}` }] };
      }
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
      if (!res.ok) {
        const text = await res.text();
        return { isError: true, content: [{ type: "text" as const, text: `API error ${res.status}: ${text}` }] };
      }
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
        list_type: z.enum(["checklist", "zakupy", "pakowanie", "terminarz", "custom"]).default("checklist").describe("Type of list: checklist (default), zakupy (shopping), pakowanie (packing), terminarz (schedule), custom"),
      },
    },
    async ({ name, list_type }) => {
      const res = await apiCall(apiWorker, devApiUrl, "POST", "/api/lists", userId, {
        name,
        list_type,
      });
      if (!res.ok) {
        const text = await res.text();
        return { isError: true, content: [{ type: "text" as const, text: `API error ${res.status}: ${text}` }] };
      }
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
      if (!res.ok) {
        const text = await res.text();
        return { isError: true, content: [{ type: "text" as const, text: `API error ${res.status}: ${text}` }] };
      }
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
      if (!res.ok) {
        const text = await res.text();
        return { isError: true, content: [{ type: "text" as const, text: `API error ${res.status}: ${text}` }] };
      }
      const data = await res.json();
      return { content: [{ type: "text", text: JSON.stringify(data, null, 2) }] };
    }
  );

  server.registerTool(
    "update_item",
    {
      description: "Update an item's title, description, dates, quantity, or move it between lists",
      inputSchema: {
        list_id: z.string().describe("The ID of the list containing the item"),
        item_id: z.string().describe("The ID of the item to update"),
        title: z.string().optional().describe("New title for the item"),
        description: z.string().nullable().optional().describe("New description (null to clear)"),
        completed: z.boolean().optional().describe("Completion state"),
        quantity: z.number().optional().describe("Target quantity"),
        actual_quantity: z.number().optional().describe("Actual quantity (auto-completes when >= quantity)"),
        unit: z.string().nullable().optional().describe("Unit of measurement (null to clear)"),
        start_date: z.string().nullable().optional().describe("Start date YYYY-MM-DD (null to clear)"),
        start_time: z.string().nullable().optional().describe("Start time HH:MM (null to clear)"),
        deadline: z.string().nullable().optional().describe("Deadline date YYYY-MM-DD (null to clear)"),
        deadline_time: z.string().nullable().optional().describe("Deadline time HH:MM (null to clear)"),
        hard_deadline: z.string().nullable().optional().describe("Hard deadline date YYYY-MM-DD (null to clear)"),
      },
    },
    async ({ list_id, item_id, ...fields }) => {
      const res = await apiCall(
        apiWorker, devApiUrl, "PUT",
        `/api/lists/${list_id}/items/${item_id}`,
        userId, fields
      );
      if (!res.ok) {
        const text = await res.text();
        return { isError: true, content: [{ type: "text" as const, text: `API error ${res.status}: ${text}` }] };
      }
      const data = await res.json();
      return { content: [{ type: "text", text: JSON.stringify(data, null, 2) }] };
    }
  );

  server.registerTool(
    "update_list",
    {
      description: "Update a list's name, description, type, or archive status",
      inputSchema: {
        list_id: z.string().describe("The ID of the list to update"),
        name: z.string().optional().describe("New name for the list"),
        description: z.string().nullable().optional().describe("New description (null to clear)"),
        list_type: z.enum(["checklist", "zakupy", "pakowanie", "terminarz", "custom"]).optional().describe("New list type"),
        archived: z.boolean().optional().describe("Archive/unarchive the list"),
      },
    },
    async ({ list_id, ...fields }) => {
      const res = await apiCall(apiWorker, devApiUrl, "PUT", `/api/lists/${list_id}`, userId, fields);
      if (!res.ok) {
        const text = await res.text();
        return { isError: true, content: [{ type: "text" as const, text: `API error ${res.status}: ${text}` }] };
      }
      const data = await res.json();
      return { content: [{ type: "text", text: JSON.stringify(data, null, 2) }] };
    }
  );

  server.registerTool(
    "move_item",
    {
      description: "Move an item to a different list",
      inputSchema: {
        item_id: z.string().describe("The ID of the item to move"),
        target_list_id: z.string().describe("The ID of the list to move the item to"),
      },
    },
    async ({ item_id, target_list_id }) => {
      const res = await apiCall(
        apiWorker, devApiUrl, "PATCH",
        `/api/items/${item_id}/move`,
        userId, { list_id: target_list_id }
      );
      if (!res.ok) {
        const text = await res.text();
        return { isError: true, content: [{ type: "text" as const, text: `API error ${res.status}: ${text}` }] };
      }
      const data = await res.json();
      return { content: [{ type: "text", text: JSON.stringify(data, null, 2) }] };
    }
  );

  server.registerTool(
    "move_list_to_container",
    {
      description: "Move a list into a container (folder/project) or remove from container",
      inputSchema: {
        list_id: z.string().describe("The ID of the list to move"),
        container_id: z.string().nullable().describe("The container ID to move into (null to remove from container)"),
      },
    },
    async ({ list_id, container_id }) => {
      const res = await apiCall(
        apiWorker, devApiUrl, "PATCH",
        `/api/lists/${list_id}/container`,
        userId, { container_id }
      );
      if (!res.ok) {
        const text = await res.text();
        return { isError: true, content: [{ type: "text" as const, text: `API error ${res.status}: ${text}` }] };
      }
      const data = await res.json();
      return { content: [{ type: "text", text: JSON.stringify(data, null, 2) }] };
    }
  );

  server.registerTool(
    "get_calendar",
    {
      description: "Get items with dates in a date range (calendar view). Returns items grouped by date with their list context.",
      inputSchema: {
        from: z.string().describe("Start date YYYY-MM-DD"),
        to: z.string().describe("End date YYYY-MM-DD"),
        field: z.enum(["start_date", "deadline", "hard_deadline"]).optional().describe("Filter by specific date field (default: all fields)"),
        mode: z.enum(["counts", "full"]).default("full").describe("'counts' returns day summaries, 'full' returns complete items"),
      },
    },
    async ({ from, to, field, mode }) => {
      const params = new URLSearchParams({ from, to, mode });
      if (field) params.set("field", field);
      const res = await apiCall(
        apiWorker, devApiUrl, "GET",
        `/api/items/calendar?${params.toString()}`,
        userId
      );
      if (!res.ok) {
        const text = await res.text();
        return { isError: true, content: [{ type: "text" as const, text: `API error ${res.status}: ${text}` }] };
      }
      const data = await res.json();
      return { content: [{ type: "text", text: JSON.stringify(data, null, 2) }] };
    }
  );

  // === TAGS ===

  server.registerTool(
    "list_tags",
    {
      description: "List all tags for the current user",
      inputSchema: {},
    },
    async () => {
      const res = await apiCall(apiWorker, devApiUrl, "GET", "/api/tags", userId);
      if (!res.ok) {
        const text = await res.text();
        return { isError: true, content: [{ type: "text" as const, text: `API error ${res.status}: ${text}` }] };
      }
      const data = await res.json();
      return { content: [{ type: "text", text: JSON.stringify(data, null, 2) }] };
    }
  );

  server.registerTool(
    "create_tag",
    {
      description: "Create a new tag",
      inputSchema: {
        name: z.string().describe("Tag name"),
        color: z.string().optional().describe("Tag color (hex, e.g. '#ff0000')"),
        parent_tag_id: z.string().optional().describe("Parent tag ID for hierarchical tags"),
      },
    },
    async ({ name, color, parent_tag_id }) => {
      const res = await apiCall(apiWorker, devApiUrl, "POST", "/api/tags", userId, { name, color, parent_tag_id });
      if (!res.ok) {
        const text = await res.text();
        return { isError: true, content: [{ type: "text" as const, text: `API error ${res.status}: ${text}` }] };
      }
      const data = await res.json();
      return { content: [{ type: "text", text: JSON.stringify(data, null, 2) }] };
    }
  );

  server.registerTool(
    "assign_tag",
    {
      description: "Assign a tag to an item or a list",
      inputSchema: {
        tag_id: z.string().describe("The tag ID to assign"),
        item_id: z.string().optional().describe("The item ID (provide either item_id or list_id)"),
        list_id: z.string().optional().describe("The list ID (provide either item_id or list_id)"),
      },
    },
    async ({ tag_id, item_id, list_id }) => {
      if (item_id) {
        const res = await apiCall(apiWorker, devApiUrl, "POST", `/api/items/${item_id}/tags`, userId, { tag_id });
        if (!res.ok) {
          const text = await res.text();
          return { isError: true, content: [{ type: "text" as const, text: `API error ${res.status}: ${text}` }] };
        }
        return { content: [{ type: "text", text: `Tag assigned to item ${item_id}` }] };
      }
      if (list_id) {
        const res = await apiCall(apiWorker, devApiUrl, "POST", `/api/lists/${list_id}/tags`, userId, { tag_id });
        if (!res.ok) {
          const text = await res.text();
          return { isError: true, content: [{ type: "text" as const, text: `API error ${res.status}: ${text}` }] };
        }
        return { content: [{ type: "text", text: `Tag assigned to list ${list_id}` }] };
      }
      return { isError: true, content: [{ type: "text" as const, text: "Provide either item_id or list_id" }] };
    }
  );

  server.registerTool(
    "remove_tag",
    {
      description: "Remove a tag from an item or a list",
      inputSchema: {
        tag_id: z.string().describe("The tag ID to remove"),
        item_id: z.string().optional().describe("The item ID (provide either item_id or list_id)"),
        list_id: z.string().optional().describe("The list ID (provide either item_id or list_id)"),
      },
    },
    async ({ tag_id, item_id, list_id }) => {
      if (item_id) {
        const res = await apiCall(apiWorker, devApiUrl, "DELETE", `/api/items/${item_id}/tags/${tag_id}`, userId);
        if (!res.ok) {
          const text = await res.text();
          return { isError: true, content: [{ type: "text" as const, text: `API error ${res.status}: ${text}` }] };
        }
        return { content: [{ type: "text", text: `Tag removed from item ${item_id}` }] };
      }
      if (list_id) {
        const res = await apiCall(apiWorker, devApiUrl, "DELETE", `/api/lists/${list_id}/tags/${tag_id}`, userId);
        if (!res.ok) {
          const text = await res.text();
          return { isError: true, content: [{ type: "text" as const, text: `API error ${res.status}: ${text}` }] };
        }
        return { content: [{ type: "text", text: `Tag removed from list ${list_id}` }] };
      }
      return { isError: true, content: [{ type: "text" as const, text: "Provide either item_id or list_id" }] };
    }
  );

  server.registerTool(
    "get_tag_items",
    {
      description: "Get all items tagged with a specific tag (optionally including child tags)",
      inputSchema: {
        tag_id: z.string().describe("The tag ID to query"),
        recursive: z.boolean().default(false).describe("Include items from child tags"),
      },
    },
    async ({ tag_id, recursive }) => {
      const params = recursive ? "?recursive=true" : "";
      const res = await apiCall(apiWorker, devApiUrl, "GET", `/api/tags/${tag_id}/items${params}`, userId);
      if (!res.ok) {
        const text = await res.text();
        return { isError: true, content: [{ type: "text" as const, text: `API error ${res.status}: ${text}` }] };
      }
      const data = await res.json();
      return { content: [{ type: "text", text: JSON.stringify(data, null, 2) }] };
    }
  );

  // === CALENDAR / DATE ===

  server.registerTool(
    "get_items_by_date",
    {
      description: "Get all items for a specific date across all lists (today view). Includes overdue items by default.",
      inputSchema: {
        date: z.string().describe("Date YYYY-MM-DD"),
        field: z.enum(["start_date", "deadline", "hard_deadline"]).optional().describe("Filter by specific date field (default: all fields)"),
        include_overdue: z.boolean().default(true).describe("Include overdue items (past deadline, not completed)"),
      },
    },
    async ({ date, field, include_overdue }) => {
      const params = new URLSearchParams({ date });
      if (field) params.set("field", field);
      if (include_overdue !== undefined) params.set("include_overdue", String(include_overdue));
      const res = await apiCall(
        apiWorker, devApiUrl, "GET",
        `/api/items/by-date?${params.toString()}`,
        userId
      );
      if (!res.ok) {
        const text = await res.text();
        return { isError: true, content: [{ type: "text" as const, text: `API error ${res.status}: ${text}` }] };
      }
      const data = await res.json();
      return { content: [{ type: "text", text: JSON.stringify(data, null, 2) }] };
    }
  );
}
