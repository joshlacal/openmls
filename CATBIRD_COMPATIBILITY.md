# Catbird OpenMLS compatibility fork

This fork moves Catbird's MLS dependency family to the released OpenMLS 0.9.2
base while retaining historical server validation and compatibility with
existing encrypted native stores and legacy post-quantum groups. The upstream
base is `8a8f83b4b61fc45f28facec25e914992b3465de6` (`v0.9.2`). Every OpenMLS crate
consumed by Catbird must use the same immutable Git revision of this fork.

## Source provenance and deliberate boundaries

The per-call leaf lifetime policy is backported from upstream PR 2257,
`044d8af408dfe3b1bfba27d423f7ad70e4312326`. It reaches GroupInfo import,
KeyPackage validation, proposals, commit processing, and deferred AppData
commit staging. The explicit `VerifyAt(u64)` policy retains Catbird's strict
`not_before < received_at < not_after` semantics. Normal `Verify` uses upstream's
wall-clock behavior, including its inclusive lower boundary. Server callers
must use the sealed receive timestamp and must never use `Skip`. Application
maximum-lifetime and minimum-remaining-time limits remain server wire policy;
this fork does not add those application-specific limits to the library.

Crypto-provider changes selectively follow upstream
`055a2b4b06b595585783ee488e28eebd62212e2a` and
`e0b21a70a55a9014b8de2772b36bf3d638adad51` (current libcrux release family). RustCrypto uses P-256/P-384 0.14,
Ed25519 3, Rand 0.10, current AEAD crates, and HPKE 0.8. Libcrux uses the current
0.0.10 cryptographic primitives and HMAC-DRBG 0.0.2. Unrelated unreleased async
storage traits, schema changes, and draft capability changes are not imported.

The opt-in `openmls_libcrux_crypto/legacy-pq-key-derivation` feature preserves
Catbird's existing MLS cipher suite 0x004D derivation. The bounded helper follows
upstream `be248d4b1306d87a641ce24ed0ffcab2b27967f6`: SHAKE256 of the input key
material yields a 32-byte seed for XWingDraft06. The MLS suite ID 0x004D maps to
HPKE KEM XWingDraft06 0x647A. It must not be confused with the obsolete HPKE KEM
0x004D. Other suites and the default HPKE 0.8 derivation retain upstream behavior.

The SQLite adapter uses rusqlite 0.41. No adapter schema, storage version,
migration identity, or serialization format changes. The bounded Refinery
manifest compatibility patch and exact official archive hashes are documented
in [vendor/CATBIRD_REFINERY_COMPATIBILITY.md](vendor/CATBIRD_REFINERY_COMPATIBILITY.md).

## Failed-write behavior

The upstream 0.9.2 sender-ratchet fix defers consumption until message validation
succeeds. This fork additionally retains only the affected sender's pre-update
state until the message-secrets write succeeds. A rejected write restores that
sender before returning the storage error, allowing retry of the same valid
message. This adds no public Clone implementation, storage schema, or wire field.

Sender-ratchet lazy initialization remains upstream behavior: the first inbound
message can change the in-memory tree representation before authentication,
without consuming a message generation. Exact unchanged-memory assertions apply
to initialized senders; cold-start checks must establish retryability separately.

## Validation boundaries

The integration uses Rust 1.99.1; the library family minimum is 1.94.0.
Historical-time tests, sender-ratchet failure tests, and native copied-store
cross-version tests are recorded in the dependency-refresh artifact directory.
Native compatibility includes original store/keychain identities, pending
commits, old/new Welcome and Commit exchange, bidirectional application messages,
corrupt-message rejection, restart acceptance, and replay rejection. These are
local cryptographic/storage compatibility checks, not production delivery or
an application UI qualification. The final integration report records exact
commands, source revisions, results, and any remaining limits.
