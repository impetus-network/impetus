-- Subscan-shape EVM explorer tables, populated by the raw-SQL writer in
-- src/sql-writer.ts (alongside the Squid TypeORM staking/holders store).
--
-- These mirror apps/explorer/prisma/schema.prisma COLUMN-FOR-COLUMN (the schema
-- the explorer's tRPC routers read via Prisma) so the explorer needs ZERO code
-- changes -- only DATABASE_URL re-pointed at this same Postgres (impetus_squid).
--
-- Idempotent (IF NOT EXISTS): executed on processor startup. A `prisma db pull`
-- against this DB must reproduce these four models in the explorer schema.
--
-- Only the 4 tables any explorer procedure actually reads are created here:
--   evm_blocks, evm_transactions, balance_accounts, evm_contracts.
-- The ~20 other tables in schema.prisma (chain_*, balance_transfers,
-- evm_transaction_receipts, evm_token*, sessions, the *_1 shards) are read by NO
-- router and are intentionally NOT created.

-- ---------------------------------------------------------------------------
-- evm_blocks  (block.* + search.block_num + metadata.blockHeight)
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS evm_blocks (
  block_num         bigint        PRIMARY KEY,
  block_hash        varchar(70),
  parent_hash       varchar(70),
  sha3_uncles       varchar(70),
  author            varchar(70),
  miner             varchar(70),
  state_root        varchar(70),
  transactions_root varchar(70),
  receipts_root     varchar(70),
  gas_used          numeric(65,0),
  gas_limit         numeric(65,0),
  extra_data        varchar(255),
  logs_bloom        text,
  timestamp         bigint,
  difficulty        numeric(65,0),
  total_difficulty  numeric(65,0),
  seal_fields       varchar(255),
  uncles            varchar(255),
  block_size        numeric(65,0),
  transaction_count integer,
  base_fee_per_gas  numeric(65,0)
);
CREATE INDEX IF NOT EXISTS idx_evm_blocks_block_hash ON evm_blocks (block_hash);

-- ---------------------------------------------------------------------------
-- evm_transactions  (tx.* + block.txList + address.txList + contract.txList
--                    + search.tx_hash + metadata.txCount)
-- transaction_id is the SOLE keyset for every tx list: deterministic
-- block_num*100000 + transaction_index (set by the writer), monotonic with
-- chain order, reorg-stable, non-null, well under 2^53.
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS evm_transactions (
  hash                     varchar(255) PRIMARY KEY,
  block_num                bigint,
  block_timestamp          bigint,
  from_address             varchar(70),
  to_address               varchar(70),
  input_data               text,
  nonce                    bigint,
  gas_limit                numeric(40,0) DEFAULT 0,
  gas_price                numeric(40,0) DEFAULT 0,
  gas_used                 numeric(40,0) DEFAULT 0,
  contract                 varchar(100),
  success                  boolean,
  r                        varchar(100),
  s                        varchar(100),
  v                        bigint,
  value                    numeric(40,0) DEFAULT 0,
  extrinsic_index          varchar(100),
  effective_gas_price      numeric(40,0) DEFAULT 0,
  max_priority_fee_per_gas numeric(40,0) DEFAULT -1,
  max_fee_per_gas          numeric(40,0) DEFAULT 0,
  cumulative_gas_used      numeric(40,0) DEFAULT 0,
  txn_type                 bigint,
  transaction_index        bigint,
  transaction_id           bigint
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_evm_transactions_transaction_id ON evm_transactions (transaction_id);
CREATE INDEX IF NOT EXISTS idx_evm_transactions_block_num ON evm_transactions (block_num);
CREATE INDEX IF NOT EXISTS idx_evm_transactions_sender ON evm_transactions (from_address);
-- Not in the introspected schema, but address.txList filters `to_address` too
-- (OR from/to) -- without this the address page does a seq scan. Mirrored into
-- apps/explorer/prisma/schema.prisma as idx_evm_transactions_receiver.
CREATE INDEX IF NOT EXISTS idx_evm_transactions_receiver ON evm_transactions (to_address);

-- ---------------------------------------------------------------------------
-- balance_accounts  (address.* + search.address + metadata.accountCount)
-- id (bigserial) is the address-list keyset (order by id desc = first-seen).
-- Sourced from the SAME System.Account read that feeds the Squid Holder entity:
--   balance <- data.free, reserved <- data.reserved, locked <- data.frozen.
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS balance_accounts (
  id       bigserial     PRIMARY KEY,
  address  varchar(100),
  nonce    bigint,
  balance  numeric(65,0),
  locked   numeric(65,0),
  reserved numeric(65,0)
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_balance_accounts_address ON balance_accounts (address);
CREATE INDEX IF NOT EXISTS idx_balance_accounts_balance ON balance_accounts (balance);
CREATE INDEX IF NOT EXISTS idx_balance_accounts_balance_address ON balance_accounts (balance, address);

-- ---------------------------------------------------------------------------
-- evm_contracts  (contract.* + search.address(contract-first) + metadata.contractCount)
-- Compound keyset (transaction_count desc, address asc). On-chain fields only;
-- verify_status / abi / source_code stay NULL until an off-chain verification
-- flow writes them (not indexer-sourceable -- matches Subscan design).
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS evm_contracts (
  address               varchar(255) PRIMARY KEY,
  abi                   json,
  source_code           text,
  creation_code         text,
  creation_bytecode     text,
  method_identifiers    json,
  deployer              varchar(100),
  block_num             bigint,
  tx_hash               varchar(70),
  deploy_at             bigint,
  verify_status         varchar(32),
  verify_type           varchar(100) DEFAULT 'SingleFile',
  contract_name         varchar(255),
  compiler_version      varchar(100),
  evm_version           varchar(100),
  external_libraries    json,
  optimize              boolean,
  optimization_runs     bigint,
  extrinsic_index       varchar(255),
  verify_time           bigint,
  transaction_count     bigint,
  precompile            bigint,
  compile_settings      json,
  eip_standard          varchar(100),
  proxy_implementation  varchar(64),
  constructor_arguments text,
  deploy_code_hash      varchar(70) NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS idx_evm_contracts_deploy_code_hash ON evm_contracts (deploy_code_hash);
CREATE INDEX IF NOT EXISTS idx_evm_contracts_transaction_count ON evm_contracts (transaction_count);
CREATE INDEX IF NOT EXISTS idx_evm_contracts_txn_count_address ON evm_contracts (transaction_count, address);
CREATE INDEX IF NOT EXISTS idx_evm_contracts_verify_status ON evm_contracts (verify_status);
