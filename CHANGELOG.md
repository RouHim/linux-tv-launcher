# [2.6.0](https://github.com/RouHim/rhinco-tv/compare/v2.5.0...v2.6.0) (2026-01-27)


### Features

* **ui:** add animated modal overlay fade-in with instant dismiss ([16a48e9](https://github.com/RouHim/rhinco-tv/commit/16a48e9f4d73f035c7c94c1ad9b52a24ecec702d))
* **ui:** add iced_anim dependency for UI animations ([78b59d6](https://github.com/RouHim/rhinco-tv/commit/78b59d6171d820d45d0af53bd9b0276977a79cf8))
* **ui:** animate category title color on selection change ([e5b8931](https://github.com/RouHim/rhinco-tv/commit/e5b8931de922705191a131d4feec0682cd3c0413))
* **ui:** animate context menu selection highlight ([aee753e](https://github.com/RouHim/rhinco-tv/commit/aee753ed5ce4aeda0586c55be667eb3c871b6f93))
* **ui:** animate item selection border and shadow glow ([c37453d](https://github.com/RouHim/rhinco-tv/commit/c37453d5e2d59725a0ab651d5b0cbd9877a727f3))

# [2.5.0](https://github.com/RouHim/rhinco-tv/compare/v2.4.1...v2.5.0) (2026-01-26)


### Features

* **snes9x:** add SNES game scanning support and update game sources ([f7a7cfe](https://github.com/RouHim/rhinco-tv/commit/f7a7cfe4d604e1ec18d4d72b76ca244ed53a6c21))

## [2.4.1](https://github.com/RouHim/rhinco-tv/compare/v2.4.0...v2.4.1) (2026-01-26)

# [2.4.0](https://github.com/RouHim/rhinco-tv/compare/v2.3.0...v2.4.0) (2026-01-26)


### Features

* **n64:** replace gopher64 with mupen64plus ([afa90ae](https://github.com/RouHim/rhinco-tv/commit/afa90ae34565d3feab52f193e77a1cd0609935b9))

# [2.3.0](https://github.com/RouHim/rhinco-tv/compare/v2.2.0...v2.3.0) (2026-01-25)


### Features

* **auth:** implement in-app sudo authentication ([d546890](https://github.com/RouHim/rhinco-tv/commit/d5468905b687bb4775fa5cbb83d7f91231b9e5be))
* **system:** implement sleep inhibition manager ([f312280](https://github.com/RouHim/rhinco-tv/commit/f31228075b66246beb8a6bc5537729d9ace7826a))
* **ui:** integrate sleep inhibition into launcher lifecycle ([258af25](https://github.com/RouHim/rhinco-tv/commit/258af250686b87b8180c690e45f51ef93a028f10))

# [2.2.0](https://github.com/RouHim/rhinco-tv/compare/v2.1.0...v2.2.0) (2026-01-24)


### Bug Fixes

* **ui:** adjust system icon size to prevent overflow ([218bec5](https://github.com/RouHim/rhinco-tv/commit/218bec564557fa9cb9574cf213c654ed5c6332ef))
* **ui:** ensure consistent scaling in status bar components ([de32fbb](https://github.com/RouHim/rhinco-tv/commit/de32fbb1a8e4dfd33ca415a35a902984681b71b4))


### Features

* **ui:** add main view vertical scrolling with controller/keyboard navigation ([97235d8](https://github.com/RouHim/rhinco-tv/commit/97235d82afb062ad6b18badb029d815435d804ff))

# [2.1.0](https://github.com/RouHim/rhinco-tv/compare/v2.0.3...v2.1.0) (2026-01-21)


### Bug Fixes

* **updater:** refactor update logic and add interactive UI ([2f3d12a](https://github.com/RouHim/rhinco-tv/commit/2f3d12aa0a06506c94b0223d29298dc84444a8e0))


### Features

* **ui:** add canvas background ([fc9f973](https://github.com/RouHim/rhinco-tv/commit/fc9f97338a17aefbbd3fa2c053a56be47d8a4aed))
* **ui:** show system battery status ([fbbdc0f](https://github.com/RouHim/rhinco-tv/commit/fbbdc0f8d4c4918e8ea6e33596952ff69255f5f2))

## [2.0.3](https://github.com/RouHim/rhinco-tv/compare/v2.0.2...v2.0.3) (2026-01-20)


### Bug Fixes

* **ui:** adjust main view padding to prevent content from being obscured by status bar and controls hint ([c47f28d](https://github.com/RouHim/rhinco-tv/commit/c47f28d23668a9be45a9a42e39519d5238d9ffc0))

## [2.0.2](https://github.com/RouHim/rhinco-tv/compare/v2.0.1...v2.0.2) (2026-01-20)

## [2.0.1](https://github.com/RouHim/rhinco-tv/compare/v2.0.0...v2.0.1) (2026-01-20)


### Bug Fixes

* **ui:** hide system update when unsupported ([6c93b19](https://github.com/RouHim/rhinco-tv/commit/6c93b195d030304d099abe9d8cec2b7c3d857b9f))

# [2.0.0](https://github.com/RouHim/rhinco-tv/compare/v1.2.0...v2.0.0) (2026-01-18)


### Bug Fixes

* **ui:** handle missing app launches gracefully ([f004ef5](https://github.com/RouHim/rhinco-tv/commit/f004ef5093234e3f7a65b5f37a644c8b8c1b4d2a))


### Features

* add Tux TV icon, update banner branding, and improve README ([11febd6](https://github.com/RouHim/rhinco-tv/commit/11febd63850df2e61fcf43f698622eb47b28bae2))
* rename application from Linux TV Launcher to RhincoTV ([24f6a28](https://github.com/RouHim/rhinco-tv/commit/24f6a281c0310d3da60e6b819235e7bf35c43083))
* replace Tux with whale shark in icon and banner ([1af11e7](https://github.com/RouHim/rhinco-tv/commit/1af11e7363de70f72082c71eaa0633546801cc1b)), closes [#0d1a2a](https://github.com/RouHim/rhinco-tv/issues/0d1a2a) [#1a2a3a](https://github.com/RouHim/rhinco-tv/issues/1a2a3a) [#5577aa](https://github.com/RouHim/rhinco-tv/issues/5577aa)


### BREAKING CHANGES

* Config directory changed from ~/.config/com/linux-tv-launcher to ~/.config/com/rhinco-tv

# [1.2.0](https://github.com/RouHim/rhinco-tv/compare/v1.1.0...v1.2.0) (2026-01-18)


### Features

* **games:** replace simple64 scan with gopher64 ([7585f92](https://github.com/RouHim/rhinco-tv/commit/7585f92ff42105535951ecc38d4b2925519c5e0b))

# [1.1.0](https://github.com/RouHim/rhinco-tv/compare/v1.0.1...v1.1.0) (2026-01-18)


### Features

* **games:** add Simple64 N64 emulator support ([0fb079c](https://github.com/RouHim/rhinco-tv/commit/0fb079cc49300988b6f6a87b1eda6591cbfbbf14))

## [1.0.1](https://github.com/RouHim/rhinco-tv/compare/v1.0.0...v1.0.1) (2026-01-17)


### Bug Fixes

* **ci:** package binaries as tar.gz and disable wayland to fix panic ([bc23277](https://github.com/RouHim/rhinco-tv/commit/bc23277d54c27a5e0085413f71bcca0178ef8d5b))
* **ci:** switch to gnu build with cross and system deps ([4d2ffb0](https://github.com/RouHim/rhinco-tv/commit/4d2ffb01d46dd3c0e43b9fecd8be1f2b4f50b595))
* **ci:** use native arm64 runner and remove cross config ([26d3cb9](https://github.com/RouHim/rhinco-tv/commit/26d3cb9dfaba37440bc6c0b4836c529c5f1927a3))

# 1.0.0 (2026-01-17)


### Bug Fixes

* add Cross.toml for musl cross-compilation dependencies ([b6dc96b](https://github.com/RouHim/rhinco-tv/commit/b6dc96b61dad24ac320ee5e21a5b076bc27a8555))
* address code review feedback ([0ffdbce](https://github.com/RouHim/rhinco-tv/commit/0ffdbce8469eea24325fe0ec46942d32e0b31879))
* battery icon alignment and build error in gamepad interval ([d74d52d](https://github.com/RouHim/rhinco-tv/commit/d74d52d3573d480ecfdcabed7a92339b1619e416))
* **ci:** add missing @semantic-release/exec dependency ([1f1d029](https://github.com/RouHim/rhinco-tv/commit/1f1d0298adb1c515db7fbe585f65a5d28a9249ea))
* **ci:** compile eudev with -fPIC to fix linker error ([9cc47f3](https://github.com/RouHim/rhinco-tv/commit/9cc47f3a1e4122023bc2cc59b0032579b614bc12))
* **ci:** install system dependencies for build and cross-compile ([893015f](https://github.com/RouHim/rhinco-tv/commit/893015f22aa65bf85487a6709f0cc595ef4ef153))
* **ci:** update workflow configuration and semantic-release setup ([7a3c830](https://github.com/RouHim/rhinco-tv/commit/7a3c830a22803fe77afeaf09df754f115fa362b8))
* cleanup unreachable code and improve error handling ([64138ef](https://github.com/RouHim/rhinco-tv/commit/64138ef7f341760346944166da83745676e1518b))
* correct YAML indentation in CI workflow ([3e41a82](https://github.com/RouHim/rhinco-tv/commit/3e41a82623136a66e2bd9884030ac6c76fbc2b94))
* **gamepad:** fix memory leak and inverted y-axis mapping ([87b5fb6](https://github.com/RouHim/rhinco-tv/commit/87b5fb6ef4fa366edec787a3b30948c4ab2bb293))
* improve GPU detection to list all installed GPUs using lspci ([7a87b0a](https://github.com/RouHim/rhinco-tv/commit/7a87b0ae5207206e61ff4173ddb6fd03ae8e0604))
* improve keyboard detection logic for controllers ([56bfc75](https://github.com/RouHim/rhinco-tv/commit/56bfc759721408fed72abbf2715cddc2f9a8b583))
* include 'Display' class in GPU detection to find secondary GPUs ([a1da455](https://github.com/RouHim/rhinco-tv/commit/a1da455c77f1b8e4f4cc53d5b37fc75634d3a318))
* install libudev-dev on ARM64 runner to resolve build failure ([d3d6493](https://github.com/RouHim/rhinco-tv/commit/d3d649313aa1f31cc587a0cc28c42156ebfc37dd))
* install pkg-config for musl builds ([90381fb](https://github.com/RouHim/rhinco-tv/commit/90381fb10750f9ff5b9b72ce43f3454ce9546c2f))
* multi-controller axis interference and unknown battery visibility ([20a30a6](https://github.com/RouHim/rhinco-tv/commit/20a30a61f8ed921e436080575c70cb5b2ff5d115))
* resolve focus manager polling issues ([93a9b02](https://github.com/RouHim/rhinco-tv/commit/93a9b02f137b400fd2241dd527c3d4688b97cb11))
* **ui:** remove duplicate update check and blocking IO in startup ([1e7654a](https://github.com/RouHim/rhinco-tv/commit/1e7654af014408a2e789ef713b0c5c20581c2667))


### Features

* Add controller bindings help modal with persistent hint ([196d82c](https://github.com/RouHim/rhinco-tv/commit/196d82cba6470ba28224309e16b232dbe704a484))
* add disk usage and ZRAM info to system info modal ([05f1bf8](https://github.com/RouHim/rhinco-tv/commit/05f1bf87258f66f23d58687b3f40518adf089b6b))
* add GPU numbering for multiple GPUs ([b26b96c](https://github.com/RouHim/rhinco-tv/commit/b26b96cf26a294888bb8ece3fba6561d35c8a718))
* add launch history keys ([dead1c7](https://github.com/RouHim/rhinco-tv/commit/dead1c7221b5c8fe9932c5fe8238b230512c682e))
* add more search paths for Proton GE detection ([c01e4c7](https://github.com/RouHim/rhinco-tv/commit/c01e4c7bfad347ba1730d646a7cacd0fbb077af7))
* Add Sansation font as static resource ([3c7ef28](https://github.com/RouHim/rhinco-tv/commit/3c7ef28e10eef3c0a42ac35eb88f16c9e883dc5d))
* Add shoulder button tab navigation with LB/RB and LT/RT ([12e6ea1](https://github.com/RouHim/rhinco-tv/commit/12e6ea1c30cab11978f4d6cfa8d2a1674fba7aff))
* add System Info modal with gaming-relevant details ([ce93017](https://github.com/RouHim/rhinco-tv/commit/ce930176d1318bd962977cb4604b9d95b819c79f))
* app removal, ui polish and parallel scanning ([fa92c86](https://github.com/RouHim/rhinco-tv/commit/fa92c86cff8caeae971411558f18a787d64dd28c))
* Block keyboard/gamepad input while game is running ([29ac8ac](https://github.com/RouHim/rhinco-tv/commit/29ac8ac30c475dfce3ee6e079c305b57d7d57e6d))
* cascading image lookup with Heroic + SteamGridDB + SearXNG ([771a0c4](https://github.com/RouHim/rhinco-tv/commit/771a0c444a39fd379089fe2ee95bd1c6c47c5ee5))
* Center tabs horizontally on screen ([9688f30](https://github.com/RouHim/rhinco-tv/commit/9688f30709fcbe5123679fc53d1b1d39ae34fa84))
* implement 'Add App' picker with XDG scanning and smart scrolling ([07932e8](https://github.com/RouHim/rhinco-tv/commit/07932e8ecd5d9964bca73f6322e6b51e50559a60))
* implement context menu and quit shortcut ([8cb0679](https://github.com/RouHim/rhinco-tv/commit/8cb06790c28ce1f059d233d087a678a5bdec7c4f))
* implement held button repeats for faster gamepad navigation ([724a0dc](https://github.com/RouHim/rhinco-tv/commit/724a0dc60fe396baedb6f794d8d8b4f15c4acc74))
* improve keyboard vs gamepad detection heuristic ([a45105d](https://github.com/RouHim/rhinco-tv/commit/a45105d39af97f0a74cc9fab69a955f6f9899234))
* improve Proton detection with version file reading ([8be1fec](https://github.com/RouHim/rhinco-tv/commit/8be1fec79ba7a7836d6d0a6725fec9d42c06b4d8))
* integrate Iced 0.14 Grid and SteamGridDB artwork ([afa2fad](https://github.com/RouHim/rhinco-tv/commit/afa2fad394ed4eb074bf490b1d4c9cf005d28815))
* migrate to musl-based static builds using rust-musl-cross ([a1e005b](https://github.com/RouHim/rhinco-tv/commit/a1e005b7d4daf208c3f79bc52f697eda1af9a05c))
* Replace SVG system icons with FontAwesome via iced_fonts ([3ab8958](https://github.com/RouHim/rhinco-tv/commit/3ab8958f7691be286eed7963cb634c1ddd995052))
* secure SteamGridDB API key and refactor config logic ([2863a58](https://github.com/RouHim/rhinco-tv/commit/2863a587144925b2413c769e1bf27be7bf41925e))
* show controller name on tooltip ([249ff7a](https://github.com/RouHim/rhinco-tv/commit/249ff7a88f04a0b2832ef17b2974cadf2f1a3293))
* show keyboard icon if controller name contains 'keyboard' ([30f745e](https://github.com/RouHim/rhinco-tv/commit/30f745ed7622ec5664b30c0d69896519dd08a9d7))
* show versions for Wine, Proton, and Proton GE in system info ([145e10c](https://github.com/RouHim/rhinco-tv/commit/145e10c45c4295b7a86667b2f6462ff7ef80aa87))
* update input and ui ([d4e0679](https://github.com/RouHim/rhinco-tv/commit/d4e0679745e16513dee51aba5403d87eccf91406))
* upgrade ureq from 2.12 to 3.1.4 ([88c36b6](https://github.com/RouHim/rhinco-tv/commit/88c36b6afab36f828524a5260b83d207d26b2b43))


### Performance Improvements

* add sccache for faster compilation ([9195dd9](https://github.com/RouHim/rhinco-tv/commit/9195dd9b137569f34084d1b9ee17e5b2d5f8cda9))
