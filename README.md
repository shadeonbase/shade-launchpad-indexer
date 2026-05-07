# shade-launchpad-indexer

Real-time, multi-launchpad event indexer for Base, part of the
[SHADE](https://shadeonbase.com/) protocol (whitepaper §4.1 — Ingestion
and Scoring).

A single Rust service subscribes to a Base WebSocket endpoint, decodes
deploy events from the major launchpad factories (Clanker, Flaunch,
Bankr, Zora), normalizes them into a uniform schema, and fans them out
to Kafka. A Postgres-backed enrichment worker then computes per-token
distribution and bytecode-risk metrics for downstream scoring.

## Components

| Path | Crate / package | Purpose |
|------|-----------------|---------|
| `crates/shade-indexer-core/`     | `shade-indexer-core`     | WS subscription, factory registry, event decoding, normalized types |
| `crates/shade-indexer-kafka/`    | `shade-indexer-kafka`    | Idempotent / transactional Kafka producer (one topic per launchpad) |
| `crates/shade-indexer-enrich/`   | `shade-indexer-enrich`   | Postgres enrichment worker: top-10 share, Gini, HHI, liquidity, bytecode flags |
| `crates/shade-indexer-bytecode/` | `shade-indexer-bytecode` | Bloom-filter pre-filter + exact scanner for malicious 4-byte selectors |
| `crates/shade-indexer-bin/`      | `shade-indexer`          | Service binary + CLI |
| `sdk/typescript/`                | `@shade/indexer-sdk`     | Consumer SDK over the Kafka topics |

## Quick start (Docker Compose)

```bash
cp .env.example .env
# edit config/indexer.toml — set [rpc].ws_url to a Base WebSocket endpoint
docker compose up -d postgres kafka
cargo run -p shade-indexer-bin -- migrate
cargo run -p shade-indexer-bin -- serve
```

Prometheus scrape endpoint defaults to `0.0.0.0:9090` (`config/indexer.toml`).

## CLI

```bash
shade-indexer serve                  # run the live ingestion pipeline
shade-indexer migrate                # apply Postgres migrations
shade-indexer backfill --from <n> --to <n>   # contiguous range, no Kafka emit
shade-indexer inspect-config         # print resolved config + factory registry
shade-indexer decode-tx <0x...>      # decode one tx and print normalized deploy(s)
```

All commands accept `--config <path>` (default `config/indexer.toml`,
overridable via `SHADE_CONFIG`).

## Kafka topics

| Topic                       | Key                  | Value                          |
|-----------------------------|----------------------|--------------------------------|
| `shade.launches.clanker`    | token address (hex)  | `NormalizedDeploy` JSON        |
| `shade.launches.flaunch`    | token address (hex)  | `NormalizedDeploy` JSON        |
| `shade.launches.bankr`      | token address (hex)  | `NormalizedDeploy` JSON        |
| `shade.launches.zora`       | token address (hex)  | `NormalizedDeploy` JSON        |

The TS SDK validates messages with [zod](https://zod.dev/) and exposes a
typed async iterator:

```ts
import { IndexerClient } from "@shade/indexer-sdk";

const client = new IndexerClient({
  brokers:    ["localhost:9092"],
  groupId:    "my-bot",
  launchpads: ["clanker", "flaunch"],
});

for await (const d of client.stream()) {
  console.log(`[${d.launchpad}] ${d.token} by ${d.deployer} @ ${d.block_number}`);
}
```

## Configuration

- `config/indexer.toml`   — RPC, Kafka, Postgres, metrics binding.
- `config/factories.toml` — per-launchpad factory address, event topic,
  ABI path. **The shipped topics are placeholders** — replace with the
  actual `keccak256("EventName(arg,arg,...)")` for each launchpad before
  mainnet use.

## Why this exists

Indexing four launchpads is commodity infrastructure — there is no
competitive advantage in keeping it private, and adopters benefit from a
single normalized schema. SHADE's differentiation lives in the scoring
engine ([`shade-score-engine`](https://github.com/shadeonbase/shade-score-engine))
and the privacy stack on top, not in the data pipe.

## Links

- Website: <https://shadeonbase.com/>
- dApp: <https://dapp.shadeonbase.com/>
- Docs: <https://docs.shadeonbase.com/>
- Whitepaper: <https://shadeonbase.com/whitepaper.pdf>
- GitHub: <https://github.com/shadeonbase>
- X (Twitter): <https://x.com/shadeonbase>
- Telegram: <https://t.me/shadeonbase>

## License

Apache-2.0
