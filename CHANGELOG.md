# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.0](https://github.com/jpalczewski/kartoteka/compare/v0.4.0...v0.5.0) (2026-03-30)


### Features

* add "move to" dropdown for moving items between lists and sublists ([26667a3](https://github.com/jpalczewski/kartoteka/commit/26667a3b59f906c5c65ac7e0869afa04ddbb4e77))
* add archive and reset functionality for lists ([7338966](https://github.com/jpalczewski/kartoteka/commit/7338966c635431478feba1a0f320e033f55eee18))
* add calendar views (month + week) and refactor frontend structure (M2) ([#13](https://github.com/jpalczewski/kartoteka/issues/13)) ([e959772](https://github.com/jpalczewski/kartoteka/commit/e9597723a9e992c4dad23bbe02f784f4915b1697))
* add ConfirmDeleteModal component with item count fetch ([3073ee9](https://github.com/jpalczewski/kartoteka/commit/3073ee992572fb002dadffb7928d41e6b7bbd4ed))
* add delete list with confirmation and optimistic update to HomePage ([69bacdb](https://github.com/jpalczewski/kartoteka/commit/69bacdbfc40bc68c087525f47bc10cf5c164085d))
* add delete list with confirmation and redirect to ListPage ([1b6da4d](https://github.com/jpalczewski/kartoteka/commit/1b6da4de4ba08d08adea008f015db47be3356a0b))
* add dev auth bypass toggled by env vars ([3bec85e](https://github.com/jpalczewski/kartoteka/commit/3bec85e9a603e2cf49a9f32d0f2b29c1edef55cb))
* add feature slices migration (lists + items columns) ([cede095](https://github.com/jpalczewski/kartoteka/commit/cede095942e9fc1a43dd12e3595482596fa19980))
* add i18n — Polish + English with device sync and MCP locale support ([#27](https://github.com/jpalczewski/kartoteka/issues/27)) ([2f7b18a](https://github.com/jpalczewski/kartoteka/commit/2f7b18a288017ebb43d170645bb2d5bd9b52aa2b))
* add list type selector, proper labels/icons, tag filter on home page ([5e2eb4c](https://github.com/jpalczewski/kartoteka/commit/5e2eb4c9bb9fe63df02e10bd5676aa5fb4b68aab))
* add on_delete prop and trash button to ListCard ([ea19060](https://github.com/jpalczewski/kartoteka/commit/ea19060f2f0dc5a41d10ab5f79176035a7d7d96c))
* add rename, move, merge actions to tags page with unified action state ([8ddf8aa](https://github.com/jpalczewski/kartoteka/commit/8ddf8aa3edee9a93b62da8255f395c9924ebb7e9))
* add sub-list sections with collapsible groups ([afb1919](https://github.com/jpalczewski/kartoteka/commit/afb1919655affaa4aa03caff8bb1eb3dd68aa2a5))
* add sublists API and move item endpoint ([2aea3e0](https://github.com/jpalczewski/kartoteka/commit/2aea3e09ce236eaa3b9d884849ad9cb93a5a8ad0))
* add tag API client functions to frontend ([57e92e7](https://github.com/jpalczewski/kartoteka/commit/57e92e7e7e1b444b74d355076c4cd8cc45b6bc85))
* add tag assignment and display on list items ([a426684](https://github.com/jpalczewski/kartoteka/commit/a4266849739878d8b391fe7ff63d87602ddc9500))
* add tag CRUD and assignment API endpoints ([1c7d2b2](https://github.com/jpalczewski/kartoteka/commit/1c7d2b2cef4df45b1bb5834f58b2877b48a15f61))
* add tag detail page, tag filtering in lists, clickable tags ([696fc14](https://github.com/jpalczewski/kartoteka/commit/696fc145b8d28b64baf20404aba327134326e944))
* add tag management for lists on home and detail pages ([fb9a797](https://github.com/jpalczewski/kartoteka/commit/fb9a797fd7c8c99cc23148df7d9643584f3ad9a3))
* add tag management page with CRUD ([fa23ef7](https://github.com/jpalczewski/kartoteka/commit/fa23ef7e99b48094cc6e5fa1dee64040342fb0cf))
* add tag merge endpoint and frontend API function ([d27c5d0](https://github.com/jpalczewski/kartoteka/commit/d27c5d00db22efbe609c6ed4d0405b2c2c7329e8))
* add tag tree utility, breadcrumb builder, and recursive fetch_tag_items param ([734df09](https://github.com/jpalczewski/kartoteka/commit/734df09b120bdc820e3caf4a2cfbcc1ac9983181))
* add Tag types and DTOs to shared crate ([50aafab](https://github.com/jpalczewski/kartoteka/commit/50aafab567a0746749cec7a739b016fc0424e2a4))
* add TagBadge and TagSelector components with CSS ([267ce9f](https://github.com/jpalczewski/kartoteka/commit/267ce9f13d97df25a872581cda166e4f56adfea7))
* add tags database schema (migration 0002) ([a5a6aec](https://github.com/jpalczewski/kartoteka/commit/a5a6aec24cbc92616e406c0d8af05bfc92b382ca))
* add Tailwind 4 + DaisyUI 5 build pipeline with Neonowa Noc theme ([5204a83](https://github.com/jpalczewski/kartoteka/commit/5204a83a04c3e10eff705af04e0448b9be31cb16))
* add Terminarz date view with sections and relative dates ([7074e8b](https://github.com/jpalczewski/kartoteka/commit/7074e8baec83fcdef4f7bdce79e39b08a4beaa4b))
* add ToastContext and ToastKind to app, stub ToastContainer ([311489b](https://github.com/jpalczewski/kartoteka/commit/311489b18dcb9c91383829cb19aea9959f36ae20))
* CI/CD pipeline for code quality & security ([#14](https://github.com/jpalczewski/kartoteka/issues/14)) ([607d75b](https://github.com/jpalczewski/kartoteka/commit/607d75bfa7dab0d38466df8d56ab1dc6153122ec))
* click-to-edit tag name on tag detail page (save on blur/enter) ([8e8b05f](https://github.com/jpalczewski/kartoteka/commit/8e8b05f86194b209878791ffdc660fca6f75ae2c))
* clickable tag pills with full path + TagList dedup + TagSelector fix ([9c4848c](https://github.com/jpalczewski/kartoteka/commit/9c4848c4908cfd39f68780e2ddce644eb9ee4702))
* clickable tag pills with full path display ([7b4ed72](https://github.com/jpalczewski/kartoteka/commit/7b4ed728acec395fd47b68c99077ef2565c9f450))
* configurable feature slice system (M3) ([#17](https://github.com/jpalczewski/kartoteka/issues/17)) ([4c79ef7](https://github.com/jpalczewski/kartoteka/commit/4c79ef7c37ebb9bbf1b530925b72eff86428869c))
* configurable log level via LOG_LEVEL env var ([#74](https://github.com/jpalczewski/kartoteka/issues/74)) ([dce43cc](https://github.com/jpalczewski/kartoteka/commit/dce43cc9a9b08e8435834177a7c7b99465ffa4b4))
* containers (folders + projects) (M5) ([#19](https://github.com/jpalczewski/kartoteka/issues/19)) ([87953df](https://github.com/jpalczewski/kartoteka/commit/87953df5f3b49a7fbfc1921c4597edb6aa59e1a2))
* feature slices, sublists, terminarz, archive, tags first-class ([cae8e69](https://github.com/jpalczewski/kartoteka/commit/cae8e69d54311c8ffc50e0e74b740b58682ef839))
* frontend preset picker, feature slices, quantity stepper ([0a36555](https://github.com/jpalczewski/kartoteka/commit/0a36555e69a1299bc97a3d506b0bb9386aa1ab2b))
* implement ToastContainer with DaisyUI alert stack ([172b7a5](https://github.com/jpalczewski/kartoteka/commit/172b7a589e0ddf56615a62dcfd32e922c01740a9))
* inline description editing for items ([f697ea4](https://github.com/jpalczewski/kartoteka/commit/f697ea4e7beb038062ebe26bf07af6c83eca0a18))
* instance settings, admin panel, invite-only registration ([#73](https://github.com/jpalczewski/kartoteka/issues/73)) ([bd616c9](https://github.com/jpalczewski/kartoteka/commit/bd616c96d00982161a8d1d31d9359588c992d536))
* item detail page with auto-save ([#38](https://github.com/jpalczewski/kartoteka/issues/38)) ([#52](https://github.com/jpalczewski/kartoteka/issues/52)) ([87a9e28](https://github.com/jpalczewski/kartoteka/commit/87a9e288ca2386ce4789dc62b37ab7d444a98eb2))
* Leptos 0.8 migration + frontend architecture refactor ([#71](https://github.com/jpalczewski/kartoteka/issues/71)) ([9459e11](https://github.com/jpalczewski/kartoteka/commit/9459e11743d7209a3f3490add7cd8624bbb98e67))
* MCP server with OAuth 2.1 + consent flow ([#25](https://github.com/jpalczewski/kartoteka/issues/25)) ([45d160d](https://github.com/jpalczewski/kartoteka/commit/45d160df70be2364fafcc1a10d79afe0815689f4))
* migrate frontend components to DaisyUI 5 + Neonowa Noc theme ([c9993b1](https://github.com/jpalczewski/kartoteka/commit/c9993b132c7d893095f67e7e15240082bf5e25b8))
* migration to drop tag category column and index ([af39f75](https://github.com/jpalczewski/kartoteka/commit/af39f75ea7b29bba7f47ae67909148d38804ee89))
* quick date buttons, time stepper, collapsible description ([b5d8c81](https://github.com/jpalczewski/kartoteka/commit/b5d8c81e6ecff011a89b49006551b51430c0d65d))
* remove category from tag handlers, add recursive filtering and cycle prevention ([2c5cbde](https://github.com/jpalczewski/kartoteka/commit/2c5cbdeb6d143d4a2e274f48e8cfe0c01dde67d4))
* remove TagCategory enum and category fields from shared models ([d859c44](https://github.com/jpalczewski/kartoteka/commit/d859c44d6d6106594bc6e3e138be2162dfeaebad))
* richer time semantics (M4) ([#18](https://github.com/jpalczewski/kartoteka/issues/18)) ([8ce6ba6](https://github.com/jpalczewski/kartoteka/commit/8ce6ba63f8cab36578ffb96e95e5ac8a53d73c5f))
* separate dev/prod/local environments with dedicated D1 databases ([ac3e55e](https://github.com/jpalczewski/kartoteka/commit/ac3e55ea16f49513adf1eb36fd5039f57c592db2))
* shared EditableTitle and EditableColor components ([58e9033](https://github.com/jpalczewski/kartoteka/commit/58e903398a377c5923085b7d2995a511549489f7))
* tag detail with breadcrumbs, recursive toggle, and subtags section ([ac4f6af](https://github.com/jpalczewski/kartoteka/commit/ac4f6af62db2c0774dbe1ad3e9df3a90717b86f8))
* tag selector with hierarchical tree and expand/collapse ([533e0f1](https://github.com/jpalczewski/kartoteka/commit/533e0f144c0c93173032ceef5a84556d4ae28885))
* tags on Terminarz items + better ItemRow tag layout ([b6a3f51](https://github.com/jpalczewski/kartoteka/commit/b6a3f51a828223145e034d3bc8408767f02a3029))
* tags page with tree view and inline add-child ([ff997c1](https://github.com/jpalczewski/kartoteka/commit/ff997c16102b25436957585e2d087332527f1643))
* today view, list descriptions, API refactor ([4d651b9](https://github.com/jpalczewski/kartoteka/commit/4d651b919968249fb660207e19d6ee75903d94c6))
* today view, list descriptions, API refactor ([baef231](https://github.com/jpalczewski/kartoteka/commit/baef231d4c50399bcc46034ed1e199280aab6d76))
* unify user settings + MCP feature validation ([#35](https://github.com/jpalczewski/kartoteka/issues/35)) ([5352074](https://github.com/jpalczewski/kartoteka/commit/53520746902434b6a3c13f679b8b9ca7f5dd0196))
* update all shared models for feature slices ([c792c4e](https://github.com/jpalczewski/kartoteka/commit/c792c4e2660674f2857e6baac45a8779b9dd6c57))
* update API handlers with feature slice fields and auto-complete ([a7d15e5](https://github.com/jpalczewski/kartoteka/commit/a7d15e5e55540cfc34ce013292495c1b27ca3c92))


### Bug Fixes

* add .into_any() to recursive TagTreeRow component ([1db14ed](https://github.com/jpalczewski/kartoteka/commit/1db14ed369441887ac6d8871b54f6706319850b7))
* add accountId and wranglerVersion to deploy workflows ([b629879](https://github.com/jpalczewski/kartoteka/commit/b629879b048d3e2e1dfd79cb0217cb3c7a834755))
* add aria-label to toast dismiss button ([2f9f8dd](https://github.com/jpalczewski/kartoteka/commit/2f9f8ddf9cf2e16fc17dec0dd5011465399c7ccb))
* add Cargo cache to release-plz PR job ([#11](https://github.com/jpalczewski/kartoteka/issues/11)) ([b0aa1dc](https://github.com/jpalczewski/kartoteka/commit/b0aa1dcb02399f17047d85d1650faa0a7481fc01))
* add compile-time env vars to release-plz workflow ([e64a8c0](https://github.com/jpalczewski/kartoteka/commit/e64a8c0e540e398f4d822f71c00cd79f9853af3d))
* add compile-time env vars to release-plz workflow ([4dd64eb](https://github.com/jpalczewski/kartoteka/commit/4dd64eb2fc49d66101763687d7712493f65b8452))
* add delete confirmation to Terminarz items ([d009391](https://github.com/jpalczewski/kartoteka/commit/d00939119d584aea7f711867eadc006d35325749))
* add type=button and aria-label for accessibility ([6e54ee2](https://github.com/jpalczewski/kartoteka/commit/6e54ee219f9d0817c2315c9381fcc9f4a5e7b36c))
* add user_id ownership checks to all API handlers ([106fbbc](https://github.com/jpalczewski/kartoteka/commit/106fbbc766656d1acdbbc1db1a5b465ed55098aa))
* add version to path dependencies for release-plz compatibility ([89f811d](https://github.com/jpalczewski/kartoteka/commit/89f811d2d1ca7bf7d577e8ff9d87d76436e60fc7))
* add version to path deps for release-plz ([69f5380](https://github.com/jpalczewski/kartoteka/commit/69f53805c2d1d552ddacfabb8046e11bfb421f48))
* auto-complete symmetry and reactive AddItemInput ([60f8402](https://github.com/jpalczewski/kartoteka/commit/60f84025f62a780d38e3d3b44946fbe288930af4))
* check HTTP status in delete_list API call ([3ebd7e9](https://github.com/jpalczewski/kartoteka/commit/3ebd7e965bf81ea8d23e280054ded04ba1875078))
* **ci:** set release-please target-branch to develop ([#77](https://github.com/jpalczewski/kartoteka/issues/77)) ([47d6f5e](https://github.com/jpalczewski/kartoteka/commit/47d6f5ec6fecd0b3608d8f2219001c793dc78e14))
* clamp quantity to &gt;=1, add date/time picker for Terminarz ([c54d692](https://github.com/jpalczewski/kartoteka/commit/c54d692a37af64d1d6404af4f7ccad1092fc777b))
* Default for ToastContext, deleting guard in modal, fetch list name in ListPage ([96fb172](https://github.com/jpalczewski/kartoteka/commit/96fb172ca75598b88936eb1919b2dd15e9e4785e))
* deploy-frontend uses empty DEV_AUTH_TOKEN for production ([232302a](https://github.com/jpalczewski/kartoteka/commit/232302aacdf192f2ab9946e01552e039574ea0fb))
* drop index before column in migration (SQLite requirement) ([76f19d6](https://github.com/jpalczewski/kartoteka/commit/76f19d6ddc904a73caafb744a64cd431fbc162d2))
* get GlooClient from context at component init, not inside spawn_local ([#75](https://github.com/jpalczewski/kartoteka/issues/75)) ([b93ed78](https://github.com/jpalczewski/kartoteka/commit/b93ed78978e08624fc4008b67cb706a44905f9bb))
* loading state and rollback position in HomePage delete flow ([72e1daf](https://github.com/jpalczewski/kartoteka/commit/72e1dafa1b781d9511d7506dd67f08da9e5c2aa0))
* make GlooClient Copy + simplify admin component client captures ([#76](https://github.com/jpalczewski/kartoteka/issues/76)) ([71bbc54](https://github.com/jpalczewski/kartoteka/commit/71bbc54707db301a7d5bbc10a70641cde6f377a1))
* mark api and frontend crates as publish = false ([0e89504](https://github.com/jpalczewski/kartoteka/commit/0e895048d0eaed726e2fa255185c54f11524ca50))
* mark api/frontend as publish=false for release-plz ([9336b9e](https://github.com/jpalczewski/kartoteka/commit/9336b9eb18cf5872e49fe0c3bb5b489ac26e9bd3))
* mark kartoteka-shared as publish = false ([33c509e](https://github.com/jpalczewski/kartoteka/commit/33c509e8df7b182404676d844a0eb5f868a92c74))
* mark shared crate as publish=false to fix release-plz ([14192dc](https://github.com/jpalczewski/kartoteka/commit/14192dccda889b3ce44743af84b4a9349a9d3ee6))
* move commit_parsers under [changelog] section in release-plz.toml ([cd054b7](https://github.com/jpalczewski/kartoteka/commit/cd054b7412cf916fd20467a66f847e4344319156))
* moved items appear instantly in target list ([ac4b6c2](https://github.com/jpalczewski/kartoteka/commit/ac4b6c234a9335be884102433e86abb6ba5c683b))
* release-plz config TOML parse error ([f18e6d6](https://github.com/jpalczewski/kartoteka/commit/f18e6d6e0db37e27a09862975e825c72ef603371))
* remove --env="" from prod deploy commands in justfile ([4d6bea1](https://github.com/jpalczewski/kartoteka/commit/4d6bea15396dcf1b96a317d4e572cb75cd1acc84))
* remove package-lock.json from git tracking ([c123e14](https://github.com/jpalczewski/kartoteka/commit/c123e145349bb6e5f9271d9ece56081bd4f6539a))
* remove publish=false from shared to unblock release-plz ([#21](https://github.com/jpalczewski/kartoteka/issues/21)) ([b3e13f6](https://github.com/jpalczewski/kartoteka/commit/b3e13f6983923c59b7b6e0d7b89585e10872b40c))
* remove redundant clone in ConfirmDeleteModal ([8ea6452](https://github.com/jpalczewski/kartoteka/commit/8ea64529f90df79e59cfb8b048e6aa998cace592))
* remove tracked package-lock.json breaking release-plz ([df265cb](https://github.com/jpalczewski/kartoteka/commit/df265cb1221fe007f76aa8baa45a9614696059d1))
* remove unused variable in tags page ([1e98eb9](https://github.com/jpalczewski/kartoteka/commit/1e98eb9bd45747afc5614c9731c4f00e5fb98135))
* replace legacy tag-group class with Tailwind mb-6 ([b3f94a3](https://github.com/jpalczewski/kartoteka/commit/b3f94a3e766995be8ece1e03388f9e1a6f4cd42e))
* save color only on close, not on each change (prevents re-render closing popup) ([8fd940b](https://github.com/jpalczewski/kartoteka/commit/8fd940b6f8e689e168b7c90cf9c032cb6789f33d))
* show "Dodaj grupę" button even when list is empty ([fc25895](https://github.com/jpalczewski/kartoteka/commit/fc258958b8ffd3513be2c3479431424ab0c7fc96))
* skip release for api/frontend crates in release-plz ([3e24e5d](https://github.com/jpalczewski/kartoteka/commit/3e24e5d1d3702bf7daefc8921146cedadb5107bc))
* skip release for api/frontend crates in release-plz ([f645de7](https://github.com/jpalczewski/kartoteka/commit/f645de72290c5a5025e10a23d73dbb026f304e84))
* today's items with past time are overdue (red) ([66c5058](https://github.com/jpalczewski/kartoteka/commit/66c5058c08dcadd20d7c3969d6bec49a03fecc5b))
* track frontend package-lock.json in git ([#70](https://github.com/jpalczewski/kartoteka/issues/70)) ([020f527](https://github.com/jpalczewski/kartoteka/commit/020f52729508f5cc47cf2fd6b813b15bb8162d5e))
* use correct DaisyUI 5 theme plugin syntax, add warning/info colors ([90fda23](https://github.com/jpalczewski/kartoteka/commit/90fda23dd9fc0601f269fd48c019bb826552c814))
* use unicode escape for wastebasket icon in ListCard ([255b622](https://github.com/jpalczewski/kartoteka/commit/255b62291268d0a6747187ea98b0bb9a038e3e25))

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
