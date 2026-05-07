-- shade-launchpad-indexer: initial schema
-- Apply with `psql -f migrations/0001_init.sql` or sqlx-cli.

CREATE TABLE IF NOT EXISTS deploys (
    id              BIGSERIAL PRIMARY KEY,
    launchpad       TEXT        NOT NULL,
    token           BYTEA       NOT NULL,
    deployer        BYTEA       NOT NULL,
    block_number    BIGINT      NOT NULL,
    block_timestamp TIMESTAMPTZ NOT NULL,
    tx_hash         BYTEA       NOT NULL,
    UNIQUE (tx_hash, token)
);

CREATE INDEX IF NOT EXISTS deploys_deployer_time
    ON deploys (deployer, block_timestamp DESC);
CREATE INDEX IF NOT EXISTS deploys_launchpad_time
    ON deploys (launchpad, block_timestamp DESC);

CREATE TABLE IF NOT EXISTS deploy_enrichment (
    deploy_id        BIGINT PRIMARY KEY REFERENCES deploys (id) ON DELETE CASCADE,
    top10_share      NUMERIC(6, 4),
    gini             NUMERIC(6, 4),
    hhi              NUMERIC(8, 4),
    liq_to_fdv_ratio NUMERIC(10, 6),
    liq_locked       BOOLEAN,
    bytecode_flags   INTEGER     NOT NULL DEFAULT 0,
    enriched_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Convenience view for downstream consumers (scoring engine, dashboards).
CREATE OR REPLACE VIEW v_deploys_enriched AS
SELECT
    d.id,
    d.launchpad,
    encode(d.token, 'hex')    AS token_hex,
    encode(d.deployer, 'hex') AS deployer_hex,
    d.block_number,
    d.block_timestamp,
    encode(d.tx_hash, 'hex')  AS tx_hash_hex,
    e.top10_share,
    e.gini,
    e.hhi,
    e.liq_to_fdv_ratio,
    e.liq_locked,
    e.bytecode_flags,
    e.enriched_at
FROM deploys d
LEFT JOIN deploy_enrichment e ON e.deploy_id = d.id;
