"""Unit tests for _common.py and smoke tests for both analysis scripts."""
import os
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(__file__))
import _common

SAMPLE_NDJSON = """\
{"timestamp":"2026-06-07T10:00:00.000Z","level":"INFO","target":"sqlx::query","fields":{"elapsed_secs":0.0005,"summary":"SELECT * FROM items WHERE"},"spans":[{"method":"GET","uri":"/leptos/get_item1135330933350608713"}],"span":{"name":"handle_request"}}
{"timestamp":"2026-06-07T10:00:01.000Z","level":"INFO","target":"sqlx::query","fields":{"elapsed_secs":0.003,"summary":"INSERT INTO items (id, title)"},"spans":[{"method":"POST","uri":"/api/items"}],"span":{"name":"create_item"}}
{"timestamp":"2026-06-07T10:00:02.000Z","level":"INFO","target":"tower_http::trace::on_response","fields":{"latency":"12 ms","status":200},"spans":[],"span":{"name":"HTTP","method":"GET","uri":"/leptos/get_item1135330933350608713"}}
{"timestamp":"2026-06-07T10:00:03.000Z","level":"INFO","target":"tower_http::trace::on_response","fields":{"latency":"5 ms","status":404},"spans":[],"span":{"name":"HTTP","method":"POST","uri":"/mcp"}}
{"timestamp":"2026-06-07T10:00:04.000Z","level":"INFO","target":"tower_http::trace::on_response","fields":{"latency":"1 ms","status":200},"spans":[],"span":{"name":"HTTP","method":"GET","uri":"/health"}}
{"timestamp":"2026-06-07T10:00:05.000Z","level":"WARN","target":"kartoteka_server","fields":{"message":"something odd","error":"timeout"},"spans":[],"span":{}}
{"timestamp":"2026-06-07T10:00:06.000Z","level":"INFO","target":"kartoteka_db::preferences","fields":{"message":"close","time.busy":"341µs","time.idle":"257µs"},"spans":[],"span":{"name":"get_locale"}}
{"timestamp":"2026-06-07T10:00:07.000Z","level":"INFO","target":"rmcp::service","fields":{"message":"received request","request":"CallTool { name: \\"list_lists\\", ... }"},"spans":[],"span":{}}
{"timestamp":"2026-06-07T10:00:08.000Z","level":"INFO","target":"rmcp::session","fields":{"stream_duration":"4.321s"},"spans":[],"span":{}}
not-json-at-all
"""


class TestStripLeptosHash(unittest.TestCase):
    def test_strips_numeric_suffix(self):
        self.assertEqual(
            _common.strip_leptos_hash("/leptos/get_item1135330933350608713"),
            "/leptos/get_item",
        )

    def test_strips_in_middle_of_url(self):
        self.assertEqual(
            _common.strip_leptos_hash("/leptos/create_list9876543210"),
            "/leptos/create_list",
        )

    def test_leaves_non_leptos_unchanged(self):
        self.assertEqual(_common.strip_leptos_hash("/api/items"), "/api/items")

    def test_leaves_leptos_without_hash_unchanged(self):
        self.assertEqual(_common.strip_leptos_hash("/leptos/get_item"), "/leptos/get_item")


class TestParseDurationMs(unittest.TestCase):
    def test_microseconds(self):
        self.assertAlmostEqual(_common.parse_duration_ms("341µs"), 0.341)

    def test_milliseconds(self):
        self.assertAlmostEqual(_common.parse_duration_ms("1.2ms"), 1.2)

    def test_seconds(self):
        self.assertAlmostEqual(_common.parse_duration_ms("3s"), 3000.0)

    def test_with_whitespace(self):
        self.assertAlmostEqual(_common.parse_duration_ms("  500µs  "), 0.5)

    def test_unknown_returns_zero(self):
        self.assertEqual(_common.parse_duration_ms("unknown"), 0.0)


class TestLoadEnvMap(unittest.TestCase):
    def test_parses_key_value(self):
        with tempfile.NamedTemporaryFile(mode="w", suffix=".local", delete=False) as f:
            f.write("myenv=abc123\n# comment\nssh_host=myhost\n")
            name = f.name
        try:
            result = _common.load_env_map(name)
            self.assertEqual(result["myenv"], "abc123")
            self.assertEqual(result["ssh_host"], "myhost")
            self.assertNotIn("# comment", result)
        finally:
            os.unlink(name)

    def test_missing_file_returns_empty(self):
        result = _common.load_env_map("/nonexistent/path/.docker-env.local")
        self.assertEqual(result, {})

    def test_known_envs_excludes_ssh_host(self):
        with tempfile.NamedTemporaryFile(mode="w", suffix=".local", delete=False) as f:
            f.write("myenv=abc123\nssh_host=myhost\n")
            name = f.name
        try:
            # Temporarily patch load_env_map path
            orig = _common.load_env_map
            _common.load_env_map = lambda path=None: orig(name)
            envs = _common.known_envs()
            self.assertIn("myenv", envs)
            self.assertNotIn("ssh_host", envs)
        finally:
            _common.load_env_map = orig
            os.unlink(name)


class TestSmokeDockerLogs(unittest.TestCase):
    def _run(self, *args):
        script = os.path.join(os.path.dirname(__file__), "docker-logs.py")
        result = subprocess.run(
            [sys.executable, script] + list(args),
            input=SAMPLE_NDJSON,
            capture_output=True,
            text=True,
        )
        return result

    def test_reads_from_stdin(self):
        r = self._run()
        self.assertEqual(r.returncode, 0)
        self.assertIn("sqlx::query", r.stdout)

    def test_filter_by_keyword(self):
        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
            f.write(SAMPLE_NDJSON)
            name = f.name
        try:
            r = self._run(name, "0", "warn")
            self.assertEqual(r.returncode, 0)
            self.assertIn("WARN", r.stdout)
            self.assertNotIn("sqlx::query", r.stdout)
        finally:
            os.unlink(name)

    def test_leptos_hash_stripped(self):
        r = self._run()
        self.assertNotIn("1135330933350608713", r.stdout)
        self.assertIn("/leptos/get_item", r.stdout)

    def test_close_enriched_with_times(self):
        r = self._run()
        self.assertIn("busy=341µs", r.stdout)

    def test_invalid_json_lines_skipped(self):
        r = self._run()
        self.assertEqual(r.returncode, 0)


class TestSmokeDockerPerf(unittest.TestCase):
    def _run(self, *args):
        script = os.path.join(os.path.dirname(__file__), "docker-perf.py")
        result = subprocess.run(
            [sys.executable, script] + list(args),
            input=SAMPLE_NDJSON,
            capture_output=True,
            text=True,
        )
        return result

    def test_reads_from_stdin(self):
        r = self._run()
        self.assertEqual(r.returncode, 0)

    def test_slow_queries_section(self):
        r = self._run()
        self.assertIn("SLOW QUERIES", r.stdout)

    def test_slow_query_detected(self):
        r = self._run()
        # 3ms INSERT should appear as slow (>1ms)
        self.assertIn("3.00ms", r.stdout)

    def test_health_endpoint_excluded(self):
        r = self._run()
        # /health should not appear in HTTP responses
        self.assertNotIn("/health", r.stdout)

    def test_mcp_404_noted(self):
        r = self._run()
        self.assertIn("session init", r.stdout)

    def test_mcp_tool_counted(self):
        r = self._run()
        self.assertIn("list_lists", r.stdout)

    def test_warnings_section(self):
        r = self._run()
        self.assertIn("WARNINGS", r.stdout)
        self.assertIn("something odd", r.stdout)

    def test_span_latencies_section(self):
        r = self._run()
        self.assertIn("SPAN LATENCIES", r.stdout)
        self.assertIn("get_locale", r.stdout)

    def test_mcp_sessions_section(self):
        r = self._run()
        self.assertIn("MCP SESSIONS", r.stdout)
        self.assertIn("4.321s", r.stdout)

    def test_leptos_hash_stripped(self):
        r = self._run()
        self.assertNotIn("1135330933350608713", r.stdout)


if __name__ == "__main__":
    unittest.main(verbosity=2)
