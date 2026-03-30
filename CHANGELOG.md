# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.1](https://github.com/jpalczewski/kartoteka/compare/v0.4.0...v0.4.1) (2026-03-30)


### Bug Fixes

* **gateway:** add partitioned cookie attribute to fix auth on Safari iOS ([#82](https://github.com/jpalczewski/kartoteka/issues/82)) ([bca8209](https://github.com/jpalczewski/kartoteka/commit/bca82099494d1c62a23536674e385b4e2dd319d6))


## [0.4.0](https://github.com/jpalczewski/kartoteka/compare/v0.3.0...v0.4.0) (2026-03-30)


### Features

* add i18n — Polish + English with device sync and MCP locale support ([#27](https://github.com/jpalczewski/kartoteka/issues/27)) ([2f7b18a](https://github.com/jpalczewski/kartoteka/commit/2f7b18a288017ebb43d170645bb2d5bd9b52aa2b))
* configurable log level via LOG_LEVEL env var ([#74](https://github.com/jpalczewski/kartoteka/issues/74)) ([dce43cc](https://github.com/jpalczewski/kartoteka/commit/dce43cc9a9b08e8435834177a7c7b99465ffa4b4))
* instance settings, admin panel, invite-only registration ([#73](https://github.com/jpalczewski/kartoteka/issues/73)) ([bd616c9](https://github.com/jpalczewski/kartoteka/commit/bd616c96d00982161a8d1d31d9359588c992d536))
* item detail page with auto-save ([#38](https://github.com/jpalczewski/kartoteka/issues/38)) ([#52](https://github.com/jpalczewski/kartoteka/issues/52)) ([87a9e28](https://github.com/jpalczewski/kartoteka/commit/87a9e288ca2386ce4789dc62b37ab7d444a98eb2))
* Leptos 0.8 migration + frontend architecture refactor ([#71](https://github.com/jpalczewski/kartoteka/issues/71)) ([9459e11](https://github.com/jpalczewski/kartoteka/commit/9459e11743d7209a3f3490add7cd8624bbb98e67))
* unify user settings + MCP feature validation ([#35](https://github.com/jpalczewski/kartoteka/issues/35)) ([5352074](https://github.com/jpalczewski/kartoteka/commit/53520746902434b6a3c13f679b8b9ca7f5dd0196))


### Bug Fixes

* add accountId and wranglerVersion to deploy workflows ([b629879](https://github.com/jpalczewski/kartoteka/commit/b629879b048d3e2e1dfd79cb0217cb3c7a834755))
* **ci:** set release-please target-branch to develop ([#77](https://github.com/jpalczewski/kartoteka/issues/77)) ([47d6f5e](https://github.com/jpalczewski/kartoteka/commit/47d6f5ec6fecd0b3608d8f2219001c793dc78e14))
* get GlooClient from context at component init, not inside spawn_local ([#75](https://github.com/jpalczewski/kartoteka/issues/75)) ([b93ed78](https://github.com/jpalczewski/kartoteka/commit/b93ed78978e08624fc4008b67cb706a44905f9bb))
* make GlooClient Copy + simplify admin component client captures ([#76](https://github.com/jpalczewski/kartoteka/issues/76)) ([71bbc54](https://github.com/jpalczewski/kartoteka/commit/71bbc54707db301a7d5bbc10a70641cde6f377a1))
* track frontend package-lock.json in git ([#70](https://github.com/jpalczewski/kartoteka/issues/70)) ([020f527](https://github.com/jpalczewski/kartoteka/commit/020f52729508f5cc47cf2fd6b813b15bb8162d5e))

## [0.4.0](https://github.com/jpalczewski/kartoteka/compare/v0.3.0...v0.4.0) (2026-03-29)


### Features

* add i18n — Polish + English with device sync and MCP locale support ([#27](https://github.com/jpalczewski/kartoteka/issues/27)) ([2f7b18a](https://github.com/jpalczewski/kartoteka/commit/2f7b18a288017ebb43d170645bb2d5bd9b52aa2b))
* item detail page with auto-save ([#38](https://github.com/jpalczewski/kartoteka/issues/38)) ([#52](https://github.com/jpalczewski/kartoteka/issues/52)) ([87a9e28](https://github.com/jpalczewski/kartoteka/commit/87a9e288ca2386ce4789dc62b37ab7d444a98eb2))
* unify user settings + MCP feature validation ([#35](https://github.com/jpalczewski/kartoteka/issues/35)) ([5352074](https://github.com/jpalczewski/kartoteka/commit/53520746902434b6a3c13f679b8b9ca7f5dd0196))

## [0.3.0](https://github.com/jpalczewski/kartoteka/compare/v0.2.0...v0.3.0) (2026-03-28)


### Features

* MCP server with OAuth 2.1 + consent flow ([#25](https://github.com/jpalczewski/kartoteka/issues/25)) ([45d160d](https://github.com/jpalczewski/kartoteka/commit/45d160df70be2364fafcc1a10d79afe0815689f4))

## [0.2.0](https://github.com/jpalczewski/kartoteka/compare/v0.1.0...v0.2.0) (2026-03-27)


### Features

* add calendar views (month + week) and refactor frontend structure (M2) ([#13](https://github.com/jpalczewski/kartoteka/issues/13)) ([e959772](https://github.com/jpalczewski/kartoteka/commit/e9597723a9e992c4dad23bbe02f784f4915b1697))
* CI/CD pipeline for code quality & security ([#14](https://github.com/jpalczewski/kartoteka/issues/14)) ([607d75b](https://github.com/jpalczewski/kartoteka/commit/607d75bfa7dab0d38466df8d56ab1dc6153122ec))
* clickable tag pills with full path + TagList dedup + TagSelector fix ([9c4848c](https://github.com/jpalczewski/kartoteka/commit/9c4848c4908cfd39f68780e2ddce644eb9ee4702))
* clickable tag pills with full path display ([7b4ed72](https://github.com/jpalczewski/kartoteka/commit/7b4ed728acec395fd47b68c99077ef2565c9f450))
* configurable feature slice system (M3) ([#17](https://github.com/jpalczewski/kartoteka/issues/17)) ([4c79ef7](https://github.com/jpalczewski/kartoteka/commit/4c79ef7c37ebb9bbf1b530925b72eff86428869c))
* containers (folders + projects) (M5) ([#19](https://github.com/jpalczewski/kartoteka/issues/19)) ([87953df](https://github.com/jpalczewski/kartoteka/commit/87953df5f3b49a7fbfc1921c4597edb6aa59e1a2))
* richer time semantics (M4) ([#18](https://github.com/jpalczewski/kartoteka/issues/18)) ([8ce6ba6](https://github.com/jpalczewski/kartoteka/commit/8ce6ba63f8cab36578ffb96e95e5ac8a53d73c5f))


### Bug Fixes

* add Cargo cache to release-plz PR job ([#11](https://github.com/jpalczewski/kartoteka/issues/11)) ([b0aa1dc](https://github.com/jpalczewski/kartoteka/commit/b0aa1dcb02399f17047d85d1650faa0a7481fc01))
* add compile-time env vars to release-plz workflow ([e64a8c0](https://github.com/jpalczewski/kartoteka/commit/e64a8c0e540e398f4d822f71c00cd79f9853af3d))
* add compile-time env vars to release-plz workflow ([4dd64eb](https://github.com/jpalczewski/kartoteka/commit/4dd64eb2fc49d66101763687d7712493f65b8452))
* mark api and frontend crates as publish = false ([0e89504](https://github.com/jpalczewski/kartoteka/commit/0e895048d0eaed726e2fa255185c54f11524ca50))
* mark api/frontend as publish=false for release-plz ([9336b9e](https://github.com/jpalczewski/kartoteka/commit/9336b9eb18cf5872e49fe0c3bb5b489ac26e9bd3))
* mark kartoteka-shared as publish = false ([33c509e](https://github.com/jpalczewski/kartoteka/commit/33c509e8df7b182404676d844a0eb5f868a92c74))
* mark shared crate as publish=false to fix release-plz ([14192dc](https://github.com/jpalczewski/kartoteka/commit/14192dccda889b3ce44743af84b4a9349a9d3ee6))
* remove publish=false from shared to unblock release-plz ([#21](https://github.com/jpalczewski/kartoteka/issues/21)) ([b3e13f6](https://github.com/jpalczewski/kartoteka/commit/b3e13f6983923c59b7b6e0d7b89585e10872b40c))

## [Unreleased]

## [0.1.1] - 2026-03-26


### Bug Fixes

- mark kartoteka-shared as publish = false
