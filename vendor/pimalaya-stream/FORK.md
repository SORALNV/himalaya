# Vendored transport provenance

Runtime source, examples, tests, manifest, README and dual MIT/Apache licenses
come from the crates.io pimalaya-stream 0.3.0 registry package. The Cargo cache
is not modified. This reduced dependency snapshot is governed by Himalaya's
root Cairn proposal/spec/log, not an independently managed upstream checkout.

The fork adds an opt-in macOS legacy-pinned feature/provider. Ordinary
rustls/native-tls providers and default selection remain unchanged. The legacy
provider requires an exact hostname and same-connection SHA-256 leaf pin,
uses TLS 1.0 only after explicit STARTTLS, and returns no transport before pin
verification. Host/pin/feature/platform errors fail closed. It adds no listener,
subprocess or global trust change. TLS 1.0 and DH1024 remain obsolete; the user
explicitly accepted this MPRG compatibility risk. Initial pin acquisition is
not an independent authentication channel.

The handshake has a ten-second total deadline enforced through an owned
duplicate-socket watchdog, cancelled and joined before returning. Loopback
fixtures cover good/changed/wrong pins, no pre-pin application bytes, invalid
profiles and silent/trickling handshake expiry. No fixture uses real mail or
credentials. The upstream dual-provider StreamKind large-enum clippy warning
is unchanged; strict legacy-only clippy and the root default checks are used.
