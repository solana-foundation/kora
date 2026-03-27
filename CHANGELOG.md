## 2.2.0-beta.7 - 2026-03-27


### Bug Fixes

- accept TransactionSigner in getPaymentInstruction source_wallet (#402)


### Documentation

- align guidance with hotfix publish flow (#407)

## 2.2.0-beta.6 - 2026-03-20


### Bug Fixes

- harden docker publish workflow run guard (#396)

- harden CPI inner-instruction reconstruction edge cases (#394)

- stabilize bundle, lighthouse header, and transfer-hook tests (#392)

- improve cherry-pick-sync to handle squash-merged sync PRs (#387)


### Features

- add swap_gas plugin + plugin infrastructure (#383)

- add cherry-pick-sync skill (#385)

## 2.0.5 - 2026-03-11


### Bug Fixes

- add repository url for npm trusted publisher provenance (#376)

## ts-sdk-v0.2.0 - 2026-03-09


### Bug Fixes

- add ESLint v9 flat config with @solana/eslint-config-solana (#371)

- update Rust version to 1.88 for time crate compatibility (#364)

- add missing signature field to SignAndSendTransactionResponse (#353)

- grant pull-request write for fork live marker comment (#360)

- guard against null head.repo when fork is deleted (#358)

- handle missing mint in transfer fee calculation and fix program_id checks (#349)

- patch remaining dependabot security vulnerabilities (#348)

- patch 6 dependabot security vulnerabilities (#346)


### Documentation

- add CONTRIBUTING.md and SECURITY.md (#340)


### Features

- implement paymaster client with kit plugin interfaces (#354)

- add kora-client and kora-operator skills (#342)

## 2.0.4 - 2026-02-03


### Bug Fixes

- clear JUPITER_API_KEY env var in jupiter oracle test (#329)

- sign if create ata ix found (cherry-pick from #267) (#327)


### Features

- add Claude skill for automated full release workflow (#330)


### Refactoring

- migrate from jupiter lite API to v3 pro API (#321)

## ts-sdk-v0.1.2 - 2026-01-28


### Documentation

- update sdk readme for release (#314)

- update examples to @solana/kit v5.x and add typechecks (#309)

- update readme with latest release and absolute logo path (#307)


### Features

- add kit plugin with typed responses (#311)

## 2.0.3 - 2026-01-21


### Hotfix

- (PRO-747) Improved Durable Nonce handling & security (#303)

## 2.0.2 - 2026-01-12


### Bug Fixes

- harden sig_verify, oracle staleness, transfer fees, and CPI stubs (#378)

- validate rpc cache redis connection at startup (#375)

- verify required env vars exist during config validation (#374)

- preserve RPC errors in transfer_transaction source ATA lookup (#373)

- add context message to ConfigError variant (#368)

- validate pubkey format in config arrays (#352)

- grant pull-request write for fork live marker comment (#359)


### Cherry-pick

- Hotfix example packages and Jupiter v3 API migration (#326)


### Documentation

- add comprehensive rustdoc for public RPC types (#356)


### Features

- add createKitKoraClient for plugin-based gasless transactions (#388)

- cache blockhash in redis (5s TTL) (#361)

- add Claude skill for automated full release workflow (#328)


### Refactoring

- reduce error boilerplate with a from-impl macro (#362)


### Testing

- migrate TS integration tests to LiteSVM (#389)

- complete FeePayerPolicy unit test matrix (#365)

- add missing retry and price bounds test coverage (#370)

- add comprehensive unit tests for instruction parsing (System, SPL, Token-2022) (#355)


### Hotfix

- (PRO-639) Fix big transaction causing error when using v0 transaction (#297)

## ts-sdk-v0.2.0-beta.4 - 2026-01-29


### Documentation

- update sdk readme for release (#314)


### Features

- add reCAPTCHA support to TypeScript client (#317)

## 2.2.0-beta.5 - 2026-02-03


### Cherry-pick

- Hotfix example packages and Jupiter v3 API migration (#326)


### Features

- add Claude skill for automated full release workflow (#328)

## ts-sdk-v0.2.0-beta.4 - 2026-01-29


### Documentation

- update sdk readme for release (#314)


### Features

- add reCAPTCHA support to TypeScript client (#317)

## 2.2.0-beta.4 - 2026-01-29


### Features

- add Lighthouse protections for fee payer protection for sign endpoints (#315)

## ts-sdk-v0.2.0-beta.3 - 2026-01-27


### Documentation

- update examples to @solana/kit v5.x and add typechecks (#309)


### Features

- add reCAPTCHA support for protected endpoints (#301)

- add kit plugin with typed responses (#311)

- update solana-keychain to v0.2.1 and add KMS and Fireblocks sig… (#308)

## 2.2.0-beta.3 - 2026-01-21


### Features

- add transactions_to_sign parameter to bundle endpoints (#302)
- implement granular usage tracking with rule-based limits (#300)

## 2.2.0-beta.2 - 2026-01-12


### Documentation

- add jito bundle example code (#295)


### Features

- (PRO-638) Estimate fee for Jito bundle endpoint (#296)


### Hotfix

- (PRO-639) Fix big transaction causing error when using v0 transaction (#297)

## 2.2.0-beta.1 - 2026-01-09


### Bug Fixes


### Documentation


### Features

- (PRO-605) Jito bundle support (#291)

### Refactoring


### Bugfix


## 2.1.0-beta.0 - 2026-01-05


### Features

- add prerelease detection for Rust and TypeScript SDK workflows

## ts-sdk-v0.1.1 - 2026-01-05


### Bug Fixes

- resolve race condition in usage tracker (#273)

- add missing signature field to SignAndSendTransactionResponse (#270)

- update readme url to docs (#269)

- sign if create ata ix found (#267)


### Documentation

- update x402 guide to x402 v2 spec (#272)

- add deploy sample & cleanup (#265)


### Refactoring

- limit access to config's singleton (#263)

## 2.0.1 - 2025-11-24


### Bug Fixes

- Readme in cargo.toml for crates.io

## v2.0.0 - 2025-11-24

### Documentation

- redirect docs content to launch.solana.com (#255)

- (PRO-278) add documentation for usage limits (#218)

- (PRO-146) add full client flow example & guide (#199)

- add CLI docs and update existing docs (#195)

- (PRO-237) release punchlist (#193)

- (PRO-220) update config docs (redis & spl22) (#181)

- (PRO-75) add TypeDoc auto-documentation (and PRO-148) (#149)

- add CONFIGURATION.md (#136)

- readme refresh (#134)

- add operator signers doc (PRO-39) (#127)

- add kora overview & operator docs (#112)

- Add Quick Start Guide for Local Development Setup (#96)


### Features

- (PRO-262) Implement usage limit feature with Redis support (#215)

- (PRO-263) Add transfer hook example and related infrastructure (#213)

- (PRO-268)  Enhance fee estimation with transfer fee calculation… (#212)

- (PRO-261) add signature verification flag to transaction methods (#208)

- allow any spl paid token flag (#175)

- Integration testing rehaul (#198)

- (PRO-246) Enhance TypeScript SDK with auto instruction parsing from b64 messages (#196)

- unit testing coverage (#190)

- (PRO-144) Add getPaymentInstruction SDK method (#192)

- (PRO-231): add get_signer_payer method (#188)

- (PRO-215) implement multi-signer support with different strategies (#184)

- (PRO-160) add fee payer balance tracking via metrics (#183)

- (PRO-162) Implement Redis caching for account data (#180)

- (PRO-212) token 2022 improvements (#179)

- Implement payment instruction fee estimation and validation (#178)

- add initialize-atas command for payment token ATAs & custo… (#173)

- add metrics collection and monitoring for Kora RPC (PRO-61) (#161)

- (PRO-149) (PRO-153) Improve process transfer transaction, so that if the ATA doesn't exists, we can process with the Mint for TransferChecked (#157)

- (PRO-70) implement compute budget handling for transaction fees (calculate priority fee instead of estimating) (#129)

- (PRO-140) enhance configuration validation with RPC options (#142)

- (PRO-141) enhance fee payer policy with burn and close account … (#140)

- add configuration validation enhancements and CLI options (#130)

- (PRO-69) Implement API Key and HMAC Authentication for Kora RPC (#119)

- (PRO-50) enhance token price calculations and fee estimation (#126)

- (PRO-70) implement compute budget handling for transaction fees (calculate priority fee instead of estimating) (#123)

- add pricing model configuration for transaction validation (PRO-56) (#114)

- enhance Kora configuration and RPC method handling (PRO-53) (#116)

- Update dependencies and refactor transaction handling to support Vers… (#108)

- Added other methods to total outflow calculation (#100)

- improved fee payer protection (#99)

- Implement TokenInterface trait and migrate tokenkeg usage (#41)

- Implement TokenInterface trait and migrate tokenkeg usage

- add openapi (#22)


### Refactoring

- Enhance margin and token value calculations with overflow p… (#252)

- (PRO-213) Inner instruction support + refactor to have support of lookup tables and inner instructions across all of Kora (#177)

- Main PR representing refactoring & code clean up & organiza… (#146)

- remove net-ts SDK and related scripts (#145)

- CI workflows and add reusable GitHub Actions

