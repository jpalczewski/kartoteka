# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
