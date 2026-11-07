# CLAUDE.md — Claude Code

@AGENTS.md

## Dokumentacja bibliotek (context7)

Projekt używa szybko ewoluujących bibliotek (Leptos 0.8, DaisyUI 5, rmcp). Przed pisaniem kodu sprawdzaj aktualne API przez context7 MCP (`mcp__context7__resolve-library-id` + `mcp__context7__query-docs`). Proaktywnie, nie czekaj na błędy.

## Delegowanie — Opus orkiestruje, nie wykonuje

- Opus (główny agent) planuje, pisze kontrakty (GH sub-issues), briefuje, robi review, decyduje i wystawia PR-y. Nie robi sam nawet prostych rzeczy (drobne edycje, poprawki z review, rutynowe zbieranie danych, odpalanie komend) — deleguje.
- Implementacja i proste zadania: 1) DeepSeek Flash przez pi (T3 `delegate_task` na instancję `pi`, `isolation: "worktree"` dla zmian w kodzie); 2) gdy pi/DeepSeek niedostępny → agent Sonnet: w T3 `delegate_task` z celem Claude Sonnet (`isolation: "worktree"`), w samym Claude Code `Agent` z `model: "sonnet"` (równoległe fale w osobnych worktree).
- Research: pi-scout (`delegate_task`, role `research`, instancja `pi-scout`); gdy niedostępny → subagent Haiku (`Agent`, `model: "haiku"`, typ `Explore`). Nie rób researchu we własnym kontekście. Brief samowystarczalny (co znaleźć, gdzie, co zwrócić), proś o fakty ze ścieżkami/URL-ami; to, na czym opiera się decyzja, weryfikuje Opus.
- Brief dla implementera zawsze zawiera: kontrakt z sub-issue, listę plików, komendy weryfikacji (`just t <crate>`, `just check-fe`), zakaz `cargo test` / `--all-targets` / `/tmp`, i `just verify` raz przed oddaniem pracy.
- Wątki koordynują się same przez wiadomości między sesjami (kolejność merge'ów, numery migracji, właściciele plików); użytkownika pytaj tylko, gdy coś się sypie lub decyzja jest jego.
