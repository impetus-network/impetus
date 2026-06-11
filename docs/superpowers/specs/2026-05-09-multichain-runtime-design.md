# Multi-chain runtime support: Impetus mainnet (388266) + Impulse testnet (322644)

## Context

Repo `apps/node/` hiện hosting một Substrate solochain duy nhất tên "Artemis"
(EVM chain id 322, token "ART", SS58 prefix 42). Cần rebrand toàn bộ và tách
thành hai mạng:

- **Mainnet "Impetus"** — token `IPT`, EVM chain id `388266`, SS58 prefix
  `11434` (= 388266 mod 16384), runtime `spec_name="impetus"`.
- **Testnet "Impulse"** — token `IPL`, EVM chain id `322644`, SS58 prefix
  `11348` (= 322644 mod 16384), runtime `spec_name="impulse"`.
- **Dev mode** — alias `--chain dev` dùng cùng runtime với Impulse, bật
  manual seal và pre-fund 5 Hardhat accounts (đúng như hiện trạng dev).

Cả ba spec đều pre-fund admin (`0xd2aE0A2139dC83Cb920e3cd7B9F640922D14b872`)
và 5 Hardhat dev accounts. Authorities tạm dùng `//Alice` (sẽ gắn vào keystore
khi đóng băng mainnet). Consensus identical (Aura + Grandpa + manual seal cho
flag `--sealing`).

`spec_name`/`impl_name` khác biệt giữa mainnet và testnet ⇒ phải có **hai
runtime build riêng biệt**, mỗi cái sản sinh một WASM blob.

## Approach: Two runtime crates + shared `runtime-common`

Pattern Polkadot/Kusama: 1 node binary chứa cả hai WASM, dispatch theo chain
spec id ở runtime. Hai runtime crate gần như duplicate (~85% lib.rs giống nhau)
nhưng khác `spec_name`, `impl_name`, `SS58Prefix`, và import WASM blob riêng.
Common crate giữ type aliases, helper traits, parameter_types không phụ thuộc
`Runtime`, custom `pallet_manual_seal` mod, và genesis helpers.

## File layout

```
apps/node/
├── runtimes/
│   ├── common/                     ← NEW
│   │   └── src/{lib.rs, precompiles.rs, gasless.rs, genesis_helpers.rs, weights/}
│   ├── impetus/                    ← NEW
│   │   └── src/{lib.rs, genesis_config_preset.rs}
│   └── impulse/                    ← NEW
│       └── src/{lib.rs, genesis_config_preset.rs}
├── node/                           giữ nguyên thư mục, sửa nội dung file
├── chain-specs/                    ← NEW
│   └── impetus.json                raw chain-spec, committed
└── runtime/                        ← XOÁ sau khi nội dung đã được tách
```

Workspace `apps/node/Cargo.toml` thay member `apps/node/runtime` bằng ba member
`runtimes/common`, `runtimes/impetus`, `runtimes/impulse`.

## Critical files — tham chiếu code hiện tại

- `apps/node/runtime/src/lib.rs:182-192` — `RuntimeVersion`/`spec_name` cần
  rebrand mỗi runtime.
- `apps/node/runtime/src/lib.rs:219` — `SS58Prefix: u8 = 42` đổi thành `u16`
  cho cả hai runtime (11434/11348 đều > 255).
- `apps/node/runtime/src/lib.rs:466-490` — custom `pallet_manual_seal` mod →
  dời sang `runtime-common`.
- `apps/node/runtime/src/genesis_config_preset.rs:15` — duplicate constant
  `ARTEMIS_CHAIN_ID = 322` → mỗi runtime có hằng riêng (`IMPETUS_CHAIN_ID =
  388266`, `IMPULSE_CHAIN_ID = 322644`).
- `apps/node/node/src/chain_spec.rs:13,22,49-56,94-124` — refactor toàn bộ
  `chain_spec.rs` thành `ChainProfile` struct + 3 builder fn. Token symbol và
  SS58 lấy từ profile thay vì hardcode. **Lưu ý**: `WASM_BINARY` là
  `Option<&'static [u8]>` từ build script ⇒ profile build qua hàm
  `fn impetus_profile() -> ChainProfile { ... wasm: impetus_runtime::
  WASM_BINARY.expect(...) }`, không phải `const`.
- `apps/node/runtime/src/gasless.rs:13,39,41,43,45,51` — hardcode
  `use crate::Runtime` và `OnChargeEVMTransaction<Runtime>`,
  `pallet_gasless_registry::Pallet::<Runtime>`. Khi dời sang common phải
  generic hoá: `pub struct GaslessEvmFee<R>(PhantomData<R>);` với bound
  `R: pallet_evm::Config + pallet_gasless_registry::Config + pallet_balances::Config`.
  `GaslessEvmRunner` ở `runtime/src/lib.rs:411` cũng cần generic hoá tương tự.
- `apps/node/runtime/src/precompiles.rs:11-44` — đã generic over
  `R: pallet_evm::Config + ...`, dời sang common không sửa.
- `apps/node/node/src/command.rs:42-53` — đổi `load_spec` để map
  `dev|impulse|testnet|impetus|mainnet` thay vì `dev|local|artemis`.
- `apps/node/node/src/service.rs:22-24,52,759-797` — tách `build_full` và
  `new_chain_ops` để dispatch theo `Network::{Impetus, Impulse}`. Bỏ alias
  `Client = FullClient<Block, RuntimeApi, HostFunctions>` (không còn default).
- `apps/node/node/src/benchmarking.rs:15` — duplicate `RemarkBuilder` /
  `TransferKeepAliveBuilder` cho mỗi runtime.
- `apps/node/node/src/client.rs`, `eth.rs` — đã generic, không sửa.

## Reusable utilities tìm thấy

- `chain_spec.rs:64-66` `admin_account()`, `chain_spec.rs:72-84`
  `mnemonic_accounts()` — chuyển nguyên sang `runtime-common::genesis_helpers`.
- `runtime/src/lib.rs:280-288` `ConsensusOnTimestampSet`, `runtime/src/lib.rs:369-382`
  `FindAuthorTruncated`, `runtime/src/lib.rs:447-458` `BaseFeeThreshold` —
  dời sang common.
- `runtime/src/lib.rs:564-585` `TransactionConverter<B>` — dời sang common.
- Type aliases `AccountId`, `Balance`, `Block`, `Header`, `Signature`,
  `SignedExtra` (`runtime/src/lib.rs:73-149`) — dời sang common, mỗi runtime
  re-export.

## Decision summary

| | Mainnet | Testnet | Dev |
|---|---|---|---|
| `--chain` alias | `impetus`, `mainnet` | `""`, `impulse`, `testnet` | `dev` |
| Display name | `Impetus` | `Impulse Testnet` | `Impulse Dev` |
| spec_id | `impetus` | `impulse` | `dev` |
| Token | `IPT` | `IPL` | `IPL` |
| EVM chainId | `388266` | `322644` | `322644` |
| SS58Prefix | `11434` | `11348` | `11348` |
| spec_name | `impetus` | `impulse` | `impulse` |
| Pre-fund | admin + 5 Hardhat | admin + 5 Hardhat | admin + 5 Hardhat |
| Authorities | `//Alice` (tạm) | `//Alice` | `//Alice` |
| Manual seal | flag-controlled | flag-controlled | bật mặc định |
| chain_type | `Live` | `Live` | `Development` |
| Runtime WASM | `impetus-runtime` | `impulse-runtime` | `impulse-runtime` |
| Raw JSON commit | `chain-specs/impetus.json` | (không) | (không) |

## Implementation outline (sắp xếp theo thứ tự PR-able)

### 1. Tạo `runtime-common` từ runtime hiện tại
- Tạo `apps/node/runtimes/common/{Cargo.toml, src/lib.rs}` với type aliases,
  helpers, custom `pallet_manual_seal`, genesis helpers, precompiles, gasless,
  weights/.
- Dời nội dung không phụ thuộc `Runtime` từ `runtime/src/lib.rs`,
  `runtime/src/precompiles.rs` (đã generic over `R`),
  `runtime/src/weights/*`, và `node/src/chain_spec.rs::{admin_account,
  mnemonic_accounts, endowed_accounts}`.
- **Generic hoá `gasless.rs`**: đổi `OnChargeEVMTransaction<Runtime>` thành
  `OnChargeEVMTransaction<R>` cho mọi callsite; đổi
  `pallet_gasless_registry::Pallet::<Runtime>` thành `::<R>`. Khai báo struct
  `pub struct GaslessEvmFee<R>(PhantomData<R>);` và
  `pub struct GaslessEvmRunner<R>(PhantomData<R>);` với bound
  `R: pallet_evm::Config + pallet_gasless_registry::Config + pallet_balances::Config + pallet_sudo::Config`.
  Mỗi runtime crate sau này set
  `type OnChargeTransaction = runtime_common::GaslessEvmFee<Self>;`.

### 2. Tạo `impetus-runtime`
- `runtimes/impetus/{Cargo.toml, build.rs}` mirror `runtime/Cargo.toml`,
  thêm `runtime-common = { path = "../common" }`.
- `src/lib.rs`: `pub use runtime_common::*;`, set `VERSION` với
  `spec_name="impetus"`, `parameter_types! { pub const SS58Prefix: u16 = 11434; }`,
  copy nguyên `impl pallet::Config for Runtime { ... }` block, `runtime!{}`
  macro, `impl_runtime_apis!{}` block.
- `src/genesis_config_preset.rs`: hằng `IMPETUS_CHAIN_ID: u64 = 388266`.

### 3. Tạo `impulse-runtime`
- Mirror impetus với `spec_name="impulse"`, `SS58Prefix=11348`,
  `IMPULSE_CHAIN_ID=322644`.

### 4. Cập nhật workspace + node Cargo.toml
- `apps/node/Cargo.toml`: workspace members += các runtime mới, –= `runtime`.
- `apps/node/node/Cargo.toml`: deps += `impetus-runtime`, `impulse-runtime`,
  `runtime-common`; bỏ `frontier-template-runtime`.

### 5. Refactor `chain_spec.rs`
- Định nghĩa `Network`, `ChainProfile`, hằng `IMPETUS`, `IMPULSE`, `DEV`,
  builders `impetus_config()`, `impulse_config()`, `development_config()`.
- `properties()` lấy từ profile.
- `genesis_patch()` dùng `serde_json::Value` literal — không phụ thuộc runtime
  types. Pre-fund admin + `mnemonic_accounts()` (5).

### 6. Refactor `command.rs::load_spec`
- Map `dev → development_config`, `"" | impulse | testnet → impulse_config`,
  `impetus | mainnet → impetus_config`, path → JSON file.

### 7. Refactor `service.rs::build_full`, `new_chain_ops`
- Đọc `config.chain_spec.id()` → match `Network::{Impetus, Impulse}`.
- Mỗi nhánh gọi `new_full::<<Runtime>::opaque::Block, <Runtime>::RuntimeApi,
  HostFunctions, _>(...)` với runtime tương ứng.
- Bỏ `pub type Client = ...`. Subcommand cần `Client` phải dispatch tự thân.

### 8. Refactor `benchmarking.rs`
- Tạo `ImpetusRemarkBuilder`/`ImpetusTransferKeepAliveBuilder` và
  `ImpulseRemarkBuilder`/`ImpulseTransferKeepAliveBuilder`.
- `command.rs::Subcommand::Benchmark` arm dispatch theo spec id để chọn cặp.

### 9. Xoá `runtime/` cũ
- Xóa thư mục, kiểm `git grep frontier-template-runtime` trên toàn repo và
  cập nhật mọi callsite (Hardhat config, Docker compose, README, AGENTS.md,
  CLAUDE.md).

### 10. Sinh và commit `chain-specs/impetus.json`
- Build node, chạy `build-spec --chain impetus --raw`, commit file.

### 11. Update docs
- `apps/node/AGENTS.md`/`CLAUDE.md`: bảng chain id/token/sudo cập nhật cho 2
  network. Repo-root `AGENTS.md` cũng cần đổi từ "Artemis 322 ART" sang bản
  multi-chain.
- `apps/node/Dockerfile`, `README.md`: chỉ dẫn `--chain` mới.

## Verification

End-to-end manual sau khi merge:

```bash
# Build
cd apps/node && cargo build --release
cd apps/node && cargo clippy --workspace -- -D warnings
cd apps/node && cargo fmt --check
cd apps/node && cargo test --workspace

# Smoke test mỗi network
./target/release/frontier-template-node --chain impetus --tmp --validator &
cast chain-id --rpc-url http://localhost:9944        # → 388266
cast balance 0xd2aE0A2139dC83Cb920e3cd7B9F640922D14b872 --rpc-url http://localhost:9944
kill %1

./target/release/frontier-template-node --chain impulse --tmp --validator &
cast chain-id --rpc-url http://localhost:9944        # → 322644
kill %1

./target/release/frontier-template-node --chain dev --sealing manual --tmp &
cast chain-id --rpc-url http://localhost:9944        # → 322644
cd packages/contracts && pnpm test                   # E2E suite vẫn pass
kill %1

# Sinh raw chain-spec mainnet
./target/release/frontier-template-node build-spec \
  --chain impetus --raw --disable-default-bootnode \
  > apps/node/chain-specs/impetus.json

# Re-run from raw spec — phải khớp genesis hash
./target/release/frontier-template-node \
  --chain ./apps/node/chain-specs/impetus.json --tmp --validator
```

## Breaking changes — checklist

- Crate name `frontier-template-runtime` biến mất → thay bằng
  `impetus-runtime`/`impulse-runtime`. Cần cập nhật mọi nơi import (E2E test
  scripts nếu có, indexer config, Hardhat).
- `--chain artemis | local` không còn nhận diện. Mọi script deploy
  (Dokploy compose, Dockerfile CMD, README, repo-root AGENTS.md) phải đổi
  sang `--chain impulse` / `--chain impetus` / `--chain dev`.
- Genesis hash mới ⇒ chain DB cũ không tương thích. Acceptable vì chưa có
  deployment production. Stop nodes, `purge-chain`, restart với spec mới.
- SS58 address format thay đổi. EVM address (H160 hex) không bị ảnh hưởng.

## Out of scope (làm sau)

- Authorities thật cho mainnet (đang dùng `//Alice`); cần thiết kế keystore
  workflow trước freeze.
- SS58 registry submission (11434/11348 hiện chỉ là deterministic modulo).
- Bootnodes và protocol id cho mainnet.
- Telemetry endpoint.
- Phân kỳ pallet giữa mainnet và testnet (cấu trúc đã hỗ trợ).
- Đổi binary name `frontier-template-node` → `impetus-node` hoặc tương tự
  (hiện giữ nguyên để giảm bề mặt thay đổi cho deployment scripts).
