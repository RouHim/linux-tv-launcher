# 1.0.0 (2026-01-17)


### Bug Fixes

* add Cross.toml for musl cross-compilation dependencies ([b6dc96b](https://github.com/RouHim/linux-tv-launcher/commit/b6dc96b61dad24ac320ee5e21a5b076bc27a8555))
* address code review feedback ([0ffdbce](https://github.com/RouHim/linux-tv-launcher/commit/0ffdbce8469eea24325fe0ec46942d32e0b31879))
* battery icon alignment and build error in gamepad interval ([d74d52d](https://github.com/RouHim/linux-tv-launcher/commit/d74d52d3573d480ecfdcabed7a92339b1619e416))
* **ci:** add missing @semantic-release/exec dependency ([1f1d029](https://github.com/RouHim/linux-tv-launcher/commit/1f1d0298adb1c515db7fbe585f65a5d28a9249ea))
* **ci:** compile eudev with -fPIC to fix linker error ([9cc47f3](https://github.com/RouHim/linux-tv-launcher/commit/9cc47f3a1e4122023bc2cc59b0032579b614bc12))
* **ci:** install system dependencies for build and cross-compile ([893015f](https://github.com/RouHim/linux-tv-launcher/commit/893015f22aa65bf85487a6709f0cc595ef4ef153))
* **ci:** update workflow configuration and semantic-release setup ([7a3c830](https://github.com/RouHim/linux-tv-launcher/commit/7a3c830a22803fe77afeaf09df754f115fa362b8))
* cleanup unreachable code and improve error handling ([64138ef](https://github.com/RouHim/linux-tv-launcher/commit/64138ef7f341760346944166da83745676e1518b))
* correct YAML indentation in CI workflow ([3e41a82](https://github.com/RouHim/linux-tv-launcher/commit/3e41a82623136a66e2bd9884030ac6c76fbc2b94))
* **gamepad:** fix memory leak and inverted y-axis mapping ([87b5fb6](https://github.com/RouHim/linux-tv-launcher/commit/87b5fb6ef4fa366edec787a3b30948c4ab2bb293))
* improve GPU detection to list all installed GPUs using lspci ([7a87b0a](https://github.com/RouHim/linux-tv-launcher/commit/7a87b0ae5207206e61ff4173ddb6fd03ae8e0604))
* improve keyboard detection logic for controllers ([56bfc75](https://github.com/RouHim/linux-tv-launcher/commit/56bfc759721408fed72abbf2715cddc2f9a8b583))
* include 'Display' class in GPU detection to find secondary GPUs ([a1da455](https://github.com/RouHim/linux-tv-launcher/commit/a1da455c77f1b8e4f4cc53d5b37fc75634d3a318))
* install libudev-dev on ARM64 runner to resolve build failure ([d3d6493](https://github.com/RouHim/linux-tv-launcher/commit/d3d649313aa1f31cc587a0cc28c42156ebfc37dd))
* install pkg-config for musl builds ([90381fb](https://github.com/RouHim/linux-tv-launcher/commit/90381fb10750f9ff5b9b72ce43f3454ce9546c2f))
* multi-controller axis interference and unknown battery visibility ([20a30a6](https://github.com/RouHim/linux-tv-launcher/commit/20a30a61f8ed921e436080575c70cb5b2ff5d115))
* resolve focus manager polling issues ([93a9b02](https://github.com/RouHim/linux-tv-launcher/commit/93a9b02f137b400fd2241dd527c3d4688b97cb11))
* **ui:** remove duplicate update check and blocking IO in startup ([1e7654a](https://github.com/RouHim/linux-tv-launcher/commit/1e7654af014408a2e789ef713b0c5c20581c2667))


### Features

* Add controller bindings help modal with persistent hint ([196d82c](https://github.com/RouHim/linux-tv-launcher/commit/196d82cba6470ba28224309e16b232dbe704a484))
* add disk usage and ZRAM info to system info modal ([05f1bf8](https://github.com/RouHim/linux-tv-launcher/commit/05f1bf87258f66f23d58687b3f40518adf089b6b))
* add GPU numbering for multiple GPUs ([b26b96c](https://github.com/RouHim/linux-tv-launcher/commit/b26b96cf26a294888bb8ece3fba6561d35c8a718))
* add launch history keys ([dead1c7](https://github.com/RouHim/linux-tv-launcher/commit/dead1c7221b5c8fe9932c5fe8238b230512c682e))
* add more search paths for Proton GE detection ([c01e4c7](https://github.com/RouHim/linux-tv-launcher/commit/c01e4c7bfad347ba1730d646a7cacd0fbb077af7))
* Add Sansation font as static resource ([3c7ef28](https://github.com/RouHim/linux-tv-launcher/commit/3c7ef28e10eef3c0a42ac35eb88f16c9e883dc5d))
* Add shoulder button tab navigation with LB/RB and LT/RT ([12e6ea1](https://github.com/RouHim/linux-tv-launcher/commit/12e6ea1c30cab11978f4d6cfa8d2a1674fba7aff))
* add System Info modal with gaming-relevant details ([ce93017](https://github.com/RouHim/linux-tv-launcher/commit/ce930176d1318bd962977cb4604b9d95b819c79f))
* app removal, ui polish and parallel scanning ([fa92c86](https://github.com/RouHim/linux-tv-launcher/commit/fa92c86cff8caeae971411558f18a787d64dd28c))
* Block keyboard/gamepad input while game is running ([29ac8ac](https://github.com/RouHim/linux-tv-launcher/commit/29ac8ac30c475dfce3ee6e079c305b57d7d57e6d))
* cascading image lookup with Heroic + SteamGridDB + SearXNG ([771a0c4](https://github.com/RouHim/linux-tv-launcher/commit/771a0c444a39fd379089fe2ee95bd1c6c47c5ee5))
* Center tabs horizontally on screen ([9688f30](https://github.com/RouHim/linux-tv-launcher/commit/9688f30709fcbe5123679fc53d1b1d39ae34fa84))
* implement 'Add App' picker with XDG scanning and smart scrolling ([07932e8](https://github.com/RouHim/linux-tv-launcher/commit/07932e8ecd5d9964bca73f6322e6b51e50559a60))
* implement context menu and quit shortcut ([8cb0679](https://github.com/RouHim/linux-tv-launcher/commit/8cb06790c28ce1f059d233d087a678a5bdec7c4f))
* implement held button repeats for faster gamepad navigation ([724a0dc](https://github.com/RouHim/linux-tv-launcher/commit/724a0dc60fe396baedb6f794d8d8b4f15c4acc74))
* improve keyboard vs gamepad detection heuristic ([a45105d](https://github.com/RouHim/linux-tv-launcher/commit/a45105d39af97f0a74cc9fab69a955f6f9899234))
* improve Proton detection with version file reading ([8be1fec](https://github.com/RouHim/linux-tv-launcher/commit/8be1fec79ba7a7836d6d0a6725fec9d42c06b4d8))
* integrate Iced 0.14 Grid and SteamGridDB artwork ([afa2fad](https://github.com/RouHim/linux-tv-launcher/commit/afa2fad394ed4eb074bf490b1d4c9cf005d28815))
* migrate to musl-based static builds using rust-musl-cross ([a1e005b](https://github.com/RouHim/linux-tv-launcher/commit/a1e005b7d4daf208c3f79bc52f697eda1af9a05c))
* Replace SVG system icons with FontAwesome via iced_fonts ([3ab8958](https://github.com/RouHim/linux-tv-launcher/commit/3ab8958f7691be286eed7963cb634c1ddd995052))
* secure SteamGridDB API key and refactor config logic ([2863a58](https://github.com/RouHim/linux-tv-launcher/commit/2863a587144925b2413c769e1bf27be7bf41925e))
* show controller name on tooltip ([249ff7a](https://github.com/RouHim/linux-tv-launcher/commit/249ff7a88f04a0b2832ef17b2974cadf2f1a3293))
* show keyboard icon if controller name contains 'keyboard' ([30f745e](https://github.com/RouHim/linux-tv-launcher/commit/30f745ed7622ec5664b30c0d69896519dd08a9d7))
* show versions for Wine, Proton, and Proton GE in system info ([145e10c](https://github.com/RouHim/linux-tv-launcher/commit/145e10c45c4295b7a86667b2f6462ff7ef80aa87))
* update input and ui ([d4e0679](https://github.com/RouHim/linux-tv-launcher/commit/d4e0679745e16513dee51aba5403d87eccf91406))
* upgrade ureq from 2.12 to 3.1.4 ([88c36b6](https://github.com/RouHim/linux-tv-launcher/commit/88c36b6afab36f828524a5260b83d207d26b2b43))


### Performance Improvements

* add sccache for faster compilation ([9195dd9](https://github.com/RouHim/linux-tv-launcher/commit/9195dd9b137569f34084d1b9ee17e5b2d5f8cda9))
