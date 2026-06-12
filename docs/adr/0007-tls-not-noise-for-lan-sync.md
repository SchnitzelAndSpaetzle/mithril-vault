# LAN Sync Transport: TLS 1.3 with Pinned Device Certificates, Not Noise

LAN Sync (see CONTEXT.md "Sync") needs an encrypted, mutually authenticated channel between paired Devices. The trust model is two raw keypairs with no PKI: each Device holds an identity keypair in the OS keychain, and Pairing pins the peer's public key after compare-and-confirm (SAS) verification.

Issue #302's research recommends the Noise Protocol Framework (Noise XX for first contact) as "the most elegant handshake for authenticating unknown peers without pre-installed static keys." That recommendation is **deliberately not followed**.

## Decision

The LAN Sync channel is **TLS 1.3 via `rustls`, over plain TCP**, with **self-signed certificates pinned to device identity**:

- Each Device's identity keypair backs a self-signed certificate; the certificate is a container for the public key, nothing more. No CAs, no chains, no expiry semantics — peer verification is exact-match pinning against the paired Devices' keys.
- Connections are mutual TLS; an unpaired peer fails the handshake and learns nothing beyond what discovery already announces — **except in pairing mode**. The first-time Pairing ceremony is necessarily a handshake with a not-yet-pinned peer: when (and only when) the user has explicitly initiated or accepted a Pairing on this Device, the verifier provisionally admits an unpinned peer so the session can complete and the SAS can be derived and displayed. Nothing is persisted at that point; trust is pinned only after both sides confirm the SAS, and an abort or mismatch discards the provisional peer entirely. Outside an active, user-initiated ceremony, unpinned peers are rejected outright.
- The Pairing SAS is derived from the TLS exporter / channel binding, so the code both users compare is cryptographically bound to the very session being established — a MITM cannot present matching codes on both screens.
- TLS 1.3 0-RTT is not used (replayable early data has no place in state-changing sync, as #302 itself notes).

## Considered Options

- **TLS 1.3 / `rustls`, pinned self-signed certificates (chosen).** `rustls` is among the most heavily audited TLS implementations anywhere, used across the Rust ecosystem, and misuse-resistant by construction: framing, rekeying, record limits, and version negotiation are the library's problem, not ours. The construction itself — device-ID-pinned certificates over TLS — is exactly what Syncthing has run at scale for a decade, so the reference model for this feature already validates it. The cost is conceptual indirection (wrapping raw keys in certificate clothing), which is boilerplate, not risk.
- **Noise XX via `snow` (issue #302's recommendation) — rejected.** A cleaner conceptual fit for "two keypairs, no certificates," and a smaller protocol surface on paper. But `snow` provides the handshake only: message framing, rekeying, size limits, and cross-version protocol negotiation become bespoke code — and this protocol must interoperate across app versions on devices that update at different times. For a team whose product is a password manager, every line of hand-rolled secure-channel plumbing is attack surface that `rustls` would have absorbed. Elegance pays when the identity model cannot fit certificates; ours fits trivially.
- **QUIC (`quinn`) instead of TCP — rejected for v1.** Multiplexed streams and connection migration buy nothing for "transfer one small file on a LAN between two stationary machines." TCP + TLS is fewer moving parts; QUIC remains open as a future transport if internet P2P (out of v1 scope) ever lands.

## Consequences

- Wire security reduces to a configuration of a mainstream, audited stack; the bespoke surface is limited to the pinning verifier and the SAS derivation.
- Device identity keys must be of a type `rustls` accepts for self-signed certs (Ed25519/ECDSA) — a constraint to fix before generating any identity keys, since they are long-lived.
- The application protocol on top (vault offers, version markers, file transfer) is versioned independently of the channel; TLS handles channel-level agility.
- If internet P2P arrives later (relay/hole-punching), the same pinned-cert mutual-TLS construction carries over unchanged — only the dial path differs.
