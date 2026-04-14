type LogLevel = "DEBUG" | "INFO" | "WARN" | "ERROR";

export function log(level: LogLevel, message: string, fields: Record<string, unknown> = {}) {
  const entry = JSON.stringify({ timestamp: new Date().toISOString(), level, message, ...fields });
  if (level === "ERROR") console.error(entry);
  else if (level === "WARN") console.warn(entry);
  else if (level === "DEBUG") console.debug(entry);
  else console.log(entry);
}
