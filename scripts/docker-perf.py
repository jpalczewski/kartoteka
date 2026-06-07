#!/usr/bin/env python3
"""Analyze query performance and HTTP latencies from NDJSON docker logs.

Usage:
    python3 scripts/docker-perf.py <env>           # SSH directly (needs scripts/.docker-env.local)
    python3 scripts/docker-perf.py <env> 300       # custom tail lines
    python3 scripts/docker-perf.py /tmp/logs.json  # from saved file

scripts/.docker-env.local is gitignored — create it with:
    <env-name>=<container-prefix>
    ssh_host=<host>
"""
import json
import re
import sys
import os

sys.path.insert(0, os.path.dirname(__file__))
from _common import known_envs, open_source, strip_leptos_hash, parse_duration_ms

from collections import Counter, defaultdict

arg1 = sys.argv[1] if len(sys.argv) > 1 else None
tail_arg = int(sys.argv[2]) if len(sys.argv) > 2 and sys.argv[2].isdigit() else 500
src = open_source(arg1, tail_arg)
lines = src.readlines()

queries = []
responses = []
mcp_tools = []
warns = []
span_busy = defaultdict(list)
mcp_sessions = []
locale_calls = 0
total_requests = 0

for line in lines:
    line = line.strip()
    if not line:
        continue
    try:
        obj = json.loads(line)
        target = obj.get("target", "")
        fields = obj.get("fields", {})
        spans = obj.get("spans", [])
        span = obj.get("span", {})
        ts = obj.get("timestamp", "")[:19].replace("T", " ")
        level = obj.get("level", "?").upper()
        span_name = span.get("name", "")

        if target == "sqlx::query":
            elapsed_ms = fields.get("elapsed_secs", 0) * 1000
            http = next((f"{s['method']} {strip_leptos_hash(s['uri'])}" for s in spans if "uri" in s), "internal")
            summary = fields.get("summary", "")[:60]
            queries.append((elapsed_ms, summary, http, span_name, ts))

        elif target == "tower_http::trace::on_response":
            latency_str = fields.get("latency", "0 ms")
            latency_ms = float(re.search(r"[\d.]+", latency_str).group())
            status = fields.get("status", 0)
            uri = strip_leptos_hash(span.get("uri", ""))
            method = span.get("method", "")
            if uri != "/health":
                responses.append((latency_ms, status, method, uri, ts))
                total_requests += 1

        elif target == "rmcp::service" and "received request" in fields.get("message", ""):
            m = re.search(r'name: "(\w+)"', fields.get("request", ""))
            if m:
                mcp_tools.append(m.group(1))

        elif "time.busy" in fields and fields.get("message") == "close":
            busy_ms = parse_duration_ms(fields["time.busy"])
            if span_name:
                span_busy[span_name].append(busy_ms)
            if span_name == "get_locale":
                locale_calls += 1

        elif "stream_duration" in fields:
            mcp_sessions.append(fields["stream_duration"])

        elif level == "WARN":
            msg = fields.get("message", "")
            error = fields.get("error", "")
            warns.append(f"{ts}  {target}  {msg}  {error}")

    except (json.JSONDecodeError, KeyError, AttributeError):
        pass

print("=== SLOW QUERIES (>1ms) ===")
slow = sorted([q for q in queries if q[0] > 1.0], reverse=True)
if slow:
    for ms, summary, http, span_name, ts in slow:
        print(f"  {ms:7.2f}ms  [{span_name:<20}]  {summary:<55}  ({http})")
else:
    print("  none")

print(f"\n=== QUERY STATS ===")
if queries:
    times = [q[0] for q in queries]
    print(f"  count={len(times)}  min={min(times):.3f}ms  avg={sum(times)/len(times):.3f}ms  "
          f"p90={sorted(times)[int(len(times)*0.9)]:.3f}ms  max={max(times):.3f}ms")
else:
    print("  no queries found")

print(f"\n=== SPAN LATENCIES (time.busy) ===")
if span_busy:
    rows = []
    for name, times in span_busy.items():
        s = sorted(times)
        rows.append((max(s), name, len(s), sum(s)/len(s), s[int(len(s)*0.9)], max(s)))
    for _, name, cnt, avg, p90, mx in sorted(rows, reverse=True):
        print(f"  {name:<30}  n={cnt:<4}  avg={avg:.2f}ms  p90={p90:.2f}ms  max={mx:.2f}ms")
else:
    print("  none")

if locale_calls > 0 and total_requests > 0:
    ratio = locale_calls / total_requests
    flag = "  ⚠ N+1 — consider caching (issue #264)" if ratio >= 0.5 else ""
    print(f"\n=== LOCALE N+1 ===")
    print(f"  get_locale calls: {locale_calls}  /  {total_requests} requests  (ratio {ratio:.1f}){flag}")

if mcp_sessions:
    print(f"\n=== MCP SESSIONS ===")
    for d in mcp_sessions:
        print(f"  session duration: {d}")

print(f"\n=== NON-HEALTH HTTP RESPONSES ===")
if responses:
    mcp_404_count = 0
    for ms, status, method, uri, ts in sorted(responses, key=lambda x: x[0], reverse=True):
        if status == 404 and uri == "/mcp":
            mcp_404_count += 1
            note = "  [session init]"
        elif status >= 400:
            note = "  ⚠"
        else:
            note = ""
        print(f"  {ts}  {method} {uri:<40} → {status}  {ms:.0f}ms{note}")
    if mcp_404_count:
        print(f"  ({mcp_404_count}x /mcp 404 = normal rmcp session init, not errors)")
else:
    print("  none")

print(f"\n=== MCP TOOL USAGE ===")
if mcp_tools:
    for tool, count in Counter(mcp_tools).most_common():
        print(f"  {tool}: {count}x")
else:
    print("  none")

print(f"\n=== WARNINGS ===")
if warns:
    for w in warns:
        print(f"  {w}")
else:
    print("  none")
