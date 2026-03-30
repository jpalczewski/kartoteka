type LogLevel = "DEBUG" | "INFO" | "WARN" | "ERROR";

export function log(level: LogLevel, message: string, fields: Record<string, unknown> = {}) {
  console.log(JSON.stringify({
    timestamp: new Date().toISOString(),
    level,
    message,
    ...fields,
  }));
}
