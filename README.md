# REC26 Rust — target 20250718.01

Clean-room Rust/Axum compatibility-server scaffold targeting Rec Room build `20250718.01` (Steam manifest `1151455856673601091`).

This project uses the public RecFlare service map/architecture as a reference, but is not a line-for-line port. The initial goal is to expose the service-discovery map, bootstrap/API routes, player/match/rooms flow, and SignalR transport in one cloud-friendly Rust container so client logs can drive compatibility work.

## Run

```bash
cargo run
```

or:

```bash
docker build -t rec26-20250718 .
docker run --rm -p 3000:3000 -e PORT=3000 -e REC26_DOMAIN=rec26.dedyn.io rec26-20250718
```

Health: `GET /__health`

## Domain model

All logical service hostnames can CNAME to the same deployment. The NS response advertises per-service HTTPS URLs under `REC26_DOMAIN`.

Implemented/meaningful RecFlare workers used as the initial REC26 domain set: ns, accounts, ai, api, auth, cdn, chat, commerce, datacollection, discovery, econ, img, leaderboard, lists, match, notify, playersettings, roomcomments, rooms.

Stub/partial workers in the public RecFlare map: cards, clubs, link, moderation, platformnotifications. RecFlare documents Moderation as currently better redirected to `api` because reporting routes live there.

Advertised but not backed by a RecFlare worker at the time this scaffold was made: bugreporting, cms, data, gamelogs, geo, roomieintegrations, storage, strings, strings-cdn, studio, thorn, videos, www.

## Safety/compatibility note

`/eac/challenge` deliberately returns 501. This server does not implement anti-cheat bypasses, TLS-pinning bypasses, signature-verification bypasses, or fabricated integrity responses. Provide only legitimate same-build artifacts/flows when required.

## Photon

Set these only when you have legitimate/self-hosted Photon-compatible infrastructure:

- `PHOTON_REALTIME_APP_ID`
- `PHOTON_VOICE_APP_ID`
- `PHOTON_CHAT_APP_ID`

The current `/player/connection-info` is a compatibility scaffold and should be replaced with the exact 20250718.01 response shape once captured/documented.

## REC26 domain areas

The current deployment intentionally uses only six public hostnames. They all
point to the same Rust container, while `src/domains.rs` assigns each hostname
to a logical area (similar to the old per-service JS files):

- `ns.rec26.dedyn.io` -> `Namespace`
- `accounts.rec26.dedyn.io` -> `Accounts`
- `api.rec26.dedyn.io` -> `Api`
- `auth.rec26.dedyn.io` -> `Auth`
- `match.rec26.dedyn.io` -> `Match`
- `rooms.rec26.dedyn.io` -> `Rooms`

For now, NS keeps the complete discovery object but aliases logical services
without their own deployed hostname to `api.rec26.dedyn.io`. This avoids
requiring dozens of DNS records during bootstrap testing. Dedicated domains can
be split out later without changing the endpoint handlers.
