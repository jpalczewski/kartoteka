# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.0](https://github.com/jpalczewski/kartoteka/compare/v1.0.0...v1.1.0) (2026-04-26)


### Features

* **domain:** validate item title and dates on create/update ([#137](https://github.com/jpalczewski/kartoteka/issues/137)) ([c6d3bde](https://github.com/jpalczewski/kartoteka/commit/c6d3bdecdd126200b7e1a0022ca03519ae545fad))


### Bug Fixes

* **ci:** allow wildcard path deps for internal workspace crates ([#138](https://github.com/jpalczewski/kartoteka/issues/138)) ([d813a80](https://github.com/jpalczewski/kartoteka/commit/d813a8055a1fa24e1fa1ffdea603396f6d14c315))

## [1.0.0](https://github.com/jpalczewski/kartoteka/compare/v0.4.1...v1.0.0) (2026-04-25)


### ⚠ BREAKING CHANGES

* small typo ([#124](https://github.com/jpalczewski/kartoteka/issues/124))

### Features

* add cursor pagination for search and collections ([#103](https://github.com/jpalczewski/kartoteka/issues/103)) ([11cad5d](https://github.com/jpalczewski/kartoteka/commit/11cad5df65fcdea0ec559f4af1ca4df709a68143))
* batch item operations and MCP placement fixes ([#102](https://github.com/jpalczewski/kartoteka/issues/102)) ([d6082ad](https://github.com/jpalczewski/kartoteka/commit/d6082ad8542a58bf926407e8a8e8925aadd9e285))
* **frontend:** add item detail links in date rows ([#99](https://github.com/jpalczewski/kartoteka/issues/99)) ([446a2f6](https://github.com/jpalczewski/kartoteka/commit/446a2f6b5d673a8fc6af88c5d0964e10e5dea863))
* **frontend:** show landing screen for unauthenticated users ([#88](https://github.com/jpalczewski/kartoteka/issues/88)) ([a94e845](https://github.com/jpalczewski/kartoteka/commit/a94e84521a54dd3acaec45bfa4cd3f157be0ca0a))
* refine tag and item detail pages ([#101](https://github.com/jpalczewski/kartoteka/issues/101)) ([073f771](https://github.com/jpalczewski/kartoteka/commit/073f7712131c40d3c26ea8370edaece343010636))
* support HTML5 drag and drop reordering ([#97](https://github.com/jpalczewski/kartoteka/issues/97)) ([fbf974c](https://github.com/jpalczewski/kartoteka/commit/fbf974cb0bcc7a22a6e3b0b07654fa5a9bba07d4))


### Bug Fixes

* **docker:** remove tailwind-input-file to prevent double Tailwind compilation ([#132](https://github.com/jpalczewski/kartoteka/issues/132)) ([fc143ca](https://github.com/jpalczewski/kartoteka/commit/fc143ca949108180b0da55b2b43655b7845db07a))
* **release:** fix release-please Cargo.toml version bumping ([#135](https://github.com/jpalczewski/kartoteka/issues/135)) ([955264e](https://github.com/jpalczewski/kartoteka/commit/955264ea54cbe5c97a66ec9d0b98b8881fe2cbb2))
* **release:** switch to generic updater for Cargo.toml version bump ([#136](https://github.com/jpalczewski/kartoteka/issues/136)) ([ff198fc](https://github.com/jpalczewski/kartoteka/commit/ff198fc2f95ebf945c5daeef3947900d27767a1c))
* repair manifest and tag UX regressions ([#95](https://github.com/jpalczewski/kartoteka/issues/95)) ([93e27dd](https://github.com/jpalczewski/kartoteka/commit/93e27dd7b0c298adaa6ffeac3d7a0c705c9e18b7))
* restore calendar item detail navigation and week layout ([#100](https://github.com/jpalczewski/kartoteka/issues/100)) ([2e32f44](https://github.com/jpalczewski/kartoteka/commit/2e32f44371b193863967e23d71276dd40c57275a))
* small typo ([#124](https://github.com/jpalczewski/kartoteka/issues/124)) ([019eba8](https://github.com/jpalczewski/kartoteka/commit/019eba83b3eb0ff3a0efb22cd0303f38fe3ee103))
* test annother approach to deployment ([f762f3b](https://github.com/jpalczewski/kartoteka/commit/f762f3b1d587d1503b14a4810d5e4e33b9e0117b))
* Validate item dates and calendar query params ([#94](https://github.com/jpalczewski/kartoteka/issues/94)) ([81217d9](https://github.com/jpalczewski/kartoteka/commit/81217d9c9e8511e44c02f9bc0dfb5718d3bc4266))
* validate service worker skip waiting messages ([#96](https://github.com/jpalczewski/kartoteka/issues/96)) ([4e0ec26](https://github.com/jpalczewski/kartoteka/commit/4e0ec26108c545616274839197ab2929f70003c8))

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
