# Catbird OpenMLS Compatibility Fork (0.9.2-pr2257)

This repository is a minimal fork of OpenMLS targeting `v0.9.2` plus:
1. A verbatim backport of upstream PR 2257 (`LeafNodeLifetimePolicy::VerifyAt`), and
2. `openmls_sqlite_storage` widened to `rusqlite 0.41` (with vendored `refinery 0.10`) under the storage-CVE exception in `docs/handoffs/2026-10-25-openmls-fork-reduction.md`.

- **Base upstream release:** `v0.9.2` (`8a8f83b4b61fc45f28facec25e914992b3465de6`)
- **Backported commit:** `044d8af408dfe3b1bfba27d423f7ad70e4312326` (PR 2257: "feat: pass the leaf node lifetime policy per call")
- **Active bookmark:** `catbird/0.9.2-pr2257`

## Purpose

The Catbird MLS delivery service (`mls-ds`) must validate historical state
transitions when processing delayed or queued messages. Specifically,
`chat_protocol/wire.rs` and `wire/direct_request.rs` validate historical transitions
using `LeafNodeLifetimePolicy::VerifyAt(sealed_receive_time)`.

Upstream PR 2257 introduced `LeafNodeLifetimePolicy::VerifyAt(u64)` and wired the
lifetime policy into message processing, commit staging, and external group creation.
PR 2257 merged to upstream `main` on 2026-10-19, but was not included in `v0.9.2`
(which branched earlier and released on 2026-10-22). This fork provides that
upstream feature on top of the stable `v0.9.2` release without carrying unreleased
`main` breaking changes (such as async storage traits), together with the `rusqlite 0.41`
storage bump required to avoid the SQLite 3.46.1 `concat_ws` CVE in `libsqlite3-sys 0.35.0`'s
bundled SQLCipher amalgamation.

## Delete-When Condition

**Delete this fork entirely and return to upstream OpenMLS** as soon as an upstream
OpenMLS release contains PR 2257 and an `openmls_sqlite_storage` dependency floor whose
bundled SQLCipher amalgamation includes SQLite >= 3.49.1 (or when upstream `refinery`
supports `rusqlite >= 0.39`).

## Lifetime Semantics

This fork adopts upstream PR 2257's verbatim lifetime verification semantics:
`not_before <= now < not_after`.
The earlier Catbird Patch 1a (strict lower bound `not_before < now`) was dropped, as
upstream's inclusive lower bound is fully compatible and non-divergent.

## Former Patches Dropped (2026-10-25 Fresh Start)

Under the 2026-10-25 architectural decision for a fresh start (no backward
compatibility required for legacy pre-release stores or legacy cipher suites):

1. **Patch 2 (Sender-ratchet rollback):** Dropped from the fork. Caching failures are
   handled in `catbird-mls` via transactional SQLite savepoints plus evict-and-reload on storage error.
2. **Patch 3 (`legacy-pq-key-derivation`):** Dropped completely. Groups use standard
   upstream derivation.
3. **Patch 4 (Newer crypto provider dependencies):** Dropped. Upstream `v0.9.2` shipped
   provider versions are used.

## Retained Storage Exception (Patch 5 — SQLCipher CVE-2025-3277 / CVE-2025-29087)

The earlier claim that `rusqlite 0.37.0` (`libsqlite3-sys 0.35.0`) had zero CVEs was
withdrawn after auditing the separate `bundled-sqlcipher` C amalgamation:
- While `libsqlite3-sys 0.35.0` bundles SQLite `3.50.2` for plain `bundled` builds (`sqlite3/sqlite3.c`),
  its `bundled-sqlcipher` feature compiles `sqlcipher/sqlite3.c` (**SQLCipher 4.6.1**, based on
  **SQLite 3.46.1**).
- `sqlcipher/sqlite3.c:135336` (`n += (argc-1)*nSep;` in `concatFuncCore`) retains the 32-bit signed
  integer overflow in `concat_ws()` covered by **CVE-2025-3277 / CVE-2025-29087** (fixed upstream in
  SQLite 3.49.1 by casting `(i64)nSep`).
- Because `libsqlite3-sys` is a links=`sqlite3` crate that permits only one version per Cargo dependency
  graph, leaving `openmls_sqlite_storage` on `rusqlite 0.37` forces `catbird-mls` (`bundled-sqlcipher`)
  onto `libsqlite3-sys 0.35.0`.
- Per the explicit storage exception in `docs/handoffs/2026-10-25-openmls-fork-reduction.md`, Patch 5 is
  retained: `openmls_sqlite_storage` uses `rusqlite 0.41` (`libsqlite3-sys 0.39.1` / `sqlcipher-src 419.0.6`,
  which bundles **SQLCipher 4.19.0** based on **SQLite 3.53.4**, where `sqlcipher.c:141533` applies
  `n += (argc-1)*(i64)nSep;`) and `vendor/refinery` `0.10.0` (with widened `rusqlite >= 0.28, <= 0.41`).

## Rebase & Conflict Resolutions (044d8af4 onto 8a8f83b4)

Upstream PR 2257 was developed against upstream `main`, which contained unreleased
async storage traits and structures not present in `v0.9.2`. Rebasing `044d8af4`
onto `8a8f83b4` produced 11 conflict hunks, resolved as follows:

1. `CHANGELOG.md`: Placed PR 2257 release notes under `## Unreleased` above `## 0.9.2`.
2. `openmls/src/framing/validation.rs`: Preserved `v0.9.2` `PendingRatchetUpdate` while
   passing `LeafNodeLifetimePolicy` to `validate_leaf_node`.
3. `openmls/src/group/mls_group/builder.rs`: Restored clean `8a8f83b4` version;
   `BranchGroupBuilder` does not exist in `v0.9.2`.
4. `openmls/src/group/mls_group/commit_builder.rs`: Threaded `LeafNodeLifetimePolicy`
   through `Initial` and `LoadedPsks` type states and added `leaf_node_lifetime_policy()`.
5. `openmls/src/group/mls_group/creation.rs`: Removed unreleased `VcExternalCommitJoinBuilder`
   hunk; wired `LeafNodeLifetimePolicy` into `ExternalCommitJoinBuilder`.
6. `openmls/src/group/mls_group/processing.rs`: Maintained synchronous `v0.9.2` signatures
   while threading `LeafNodeLifetimePolicy` through `process_message_with_lifetime_policy`,
   `process_unverified_message`, `process_verified_message`, and `stage_commit`.
7. `openmls/src/group/public_group/mod.rs`: Maintained synchronous `from_external_with_lifetime_policy`
   (removed `async` and `#[maybe_async]` from unreleased main).
8. `openmls/src/group/public_group/validation.rs`: Wired `lifetime_policy` to `validate_add_proposals`
   and `validate_leaf_node`.
9. `openmls/src/key_packages/key_package_in.rs`: `validate` delegates to
   `validate_with_lifetime_policy(..., LeafNodeLifetimePolicy::Verify)`.
10. `openmls/src/messages/mod.rs`: Re-exported `LeafNodeLifetimePolicy` from `group`.
11. `openmls/src/messages/proposals_in.rs` & `tests/proposals.rs`: Validated ciphersuite
    per `v0.9.2` conventions (omitting `unverified_ciphersuite` and its unreleased unit test);
    in `tests/leaf_node_lifetime_policy.rs`, omitted `a_branch_takes_the_policy` which tested
    the unreleased `BranchGroupBuilder`.
