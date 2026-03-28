export interface ApiContext {
  apiWorker: Fetcher | undefined;
  devApiUrl: string | undefined;
  userId: string;
}

export type ToolResult = {
  content: { type: "text"; text: string }[];
  isError?: boolean;
};

export async function apiCall(
  api: ApiContext,
  method: string,
  path: string,
  body?: unknown
): Promise<Response> {
  const headers = new Headers({
    "Content-Type": "application/json",
    "X-User-Id": api.userId,
  });
  const init: RequestInit = { method, headers };
  if (body !== undefined) init.body = JSON.stringify(body);

  if (api.devApiUrl) return fetch(`${api.devApiUrl}${path}`, init);
  return api.apiWorker!.fetch(new Request(`https://api-worker${path}`, init));
}

export async function callTool(
  api: ApiContext,
  method: string,
  path: string,
  body?: unknown
): Promise<ToolResult> {
  const res = await apiCall(api, method, path, body);
  if (!res.ok) {
    const text = await res.text();
    return errorResult(`API error ${res.status}: ${text}`);
  }
  const data = await res.json();
  return jsonResult(data);
}

export function jsonResult(data: unknown): ToolResult {
  return { content: [{ type: "text", text: JSON.stringify(data, null, 2) }] };
}

export function textResult(text: string): ToolResult {
  return { content: [{ type: "text", text }] };
}

export function errorResult(text: string): ToolResult {
  return { isError: true, content: [{ type: "text", text }] };
}

export async function ensureFeatures(
  api: ApiContext,
  listId: string,
  fields: Record<string, unknown>
): Promise<void> {
  const hasDateField =
    fields.start_date !== undefined ||
    fields.deadline !== undefined ||
    fields.hard_deadline !== undefined ||
    fields.start_time !== undefined ||
    fields.deadline_time !== undefined;
  const hasQuantity =
    fields.quantity !== undefined ||
    fields.actual_quantity !== undefined ||
    fields.unit !== undefined;

  if (hasDateField) {
    await apiCall(api, "POST", `/api/lists/${listId}/features/deadlines`, { config: {} });
  }
  if (hasQuantity) {
    await apiCall(api, "POST", `/api/lists/${listId}/features/quantity`, { config: {} });
  }
}
