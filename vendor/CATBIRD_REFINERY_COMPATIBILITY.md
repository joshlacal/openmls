# Refinery compatibility for rusqlite 0.41

These three directories contain the official published Refinery 0.10.0 crates.
They preserve the OpenMLS SQLite adapter's migration behavior while permitting
rusqlite 0.41, the SQLite version used by the refreshed native client. The only
non-path manifest change extends refinery-core's rusqlite upper bound from
0.40 to 0.41. Rust source, migration identities, checksums, codecs, and schema
are unchanged. Cargo.toml.orig and the upstream licenses are preserved.

| Official crate archive | SHA-256 |
| --- | --- |
| https://static.crates.io/crates/refinery/refinery-0.10.0.crate | `8e48a123dfddb4e33b888a233e3dd8f428d0dadbe8a2302177fcad504b9fb0e3` |
| https://static.crates.io/crates/refinery-core/refinery-core-0.10.0.crate | `c5357919ea1ec6b73b47471c5182cdb8fa09f679b0bcebc435066df5539a8081` |
| https://static.crates.io/crates/refinery-macros/refinery-macros-0.10.0.crate | `623c53cdc704ababffe5361be8cb726f874fcd4e97b945741aa69b15d0406f36` |

The OpenMLS adapter and each vendored member refer directly to their sibling
paths, because a dependency's root [patch] section is not inherited by Cargo
consumers. This keeps the adapter and downstream clients on the same source
family without requiring each consumer to repeat a patch.

Replace these directories with upstream releases once they accept rusqlite
0.41, then repeat copied-store reopen and migration checks before release.
