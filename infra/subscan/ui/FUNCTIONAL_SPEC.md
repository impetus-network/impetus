# Subscan Block Explorer - Functional Specification

Blockchain block explorer for an EVM-compatible Substrate solochain (Frontier). The chain exposes two execution layers: **Substrate** (native runtime) and **EVM** (smart contracts). The UI adapts dynamically based on which layers are enabled via a metadata API flag (`enable_substrate`, `enable_evm`).

Chain: Artemis | Chain ID: 322 | Token: ART (18 decimals) | Gasless

---

## Global

### Global Search

A unified search bar available on every page. The user selects a search category from a dropdown, types a value, and presses Enter.

Searchable categories when Substrate is enabled:
- Substrate Block (by block number)
- Substrate Extrinsic (by extrinsic index or hash)
- Substrate Event (by event index)
- Substrate Account (by SS58 address)

Searchable categories when EVM is enabled:
- EVM Block (by block number)
- EVM Transaction (by tx hash)
- Smart Contract (by contract address)
- EVM Account (by 0x address)

### Navigation

Top-level navigation adapts based on enabled layers:

When both layers enabled: two dropdown menus (Substrate, Smart Contract) plus direct links to Extrinsic and Transaction.

Substrate dropdown items: Block, Extrinsic, Account, Event, Transfer.

Smart Contract dropdown items: Block, Transaction, Account, Contract, ERC-20, ERC-721.

When only one layer enabled: flat navigation links for that layer's entities.

### Data Context

All pages share a global data context that provides:
- `metadata`: network info, counts, feature flags
- `token`: native token info (symbol, decimals)

---

## Screen 1: Home Page

**Route:** `/`

Dashboard showing network overview statistics and latest activity.

### Substrate Section (when `enable_substrate`)

Summary cards:
- Substrate Block: total finalized block number, links to `/sub/block`
- Extrinsic: total extrinsic count, links to `/sub/extrinsic`
- Account: total account count, links to `/sub/account`
- Transfer: total transfer count, links to `/sub/transfer`

Latest activity lists:
- Recent Substrate blocks (when single-layer mode)
- Recent extrinsics

### EVM Section (when `enable_evm`)

Summary cards:
- Smart Contract Block: total block number, links to `/block`
- Transaction: total transaction count, links to `/tx`
- Smart Contract Account: total EVM account count, links to `/address`
- Smart Contract: total contract count, links to `/contract`

Latest activity lists:
- Recent EVM blocks (when single-layer mode)
- Recent transactions

---

## Screen 2: Substrate Block List

**Route:** `/sub/block`

Paginated table of Substrate blocks.

Table columns:
- Block number (links to block detail)
- Timestamp
- Hash
- Extrinsic count
- Event count
- Validator
- Finalized status

Pagination: cursor-based (has_next_page, has_previous_page).

---

## Screen 3: Substrate Block Detail

**Route:** `/sub/block/[id]`

**Lookup:** by block number.

Block info fields:
- Timestamp
- Blocktime (relative, e.g. "2 minutes ago")
- Status: Finalized / Unfinalized
- Hash (copyable)
- Parent Hash (copyable)
- State Root
- Extrinsics Root
- Collator address
- Spec Version

Tabbed sub-sections:
- **Extrinsics tab:** paginated table of extrinsics in this block
- **Events tab:** paginated table of events in this block
- **Log tab:** list of block logs (log_type, data)

---

## Screen 4: Substrate Extrinsic List

**Route:** `/sub/extrinsic`

Paginated table of extrinsics.

Table columns:
- Extrinsic index (links to detail)
- Block number (links to block detail)
- Extrinsic hash
- Timestamp
- Action (module + function)
- Result (Success/Failed)
- Fee

Pagination: cursor-based.

---

## Screen 5: Substrate Extrinsic Detail

**Route:** `/sub/extrinsic/[id]`

**Lookup:** by extrinsic index (e.g. `12345-2`) or extrinsic hash.

Extrinsic info fields:
- Timestamp
- Blocktime (relative)
- Block number (links to block detail)
- Lifetime (birth block - death block, or "-" if immortal)
- Extrinsic Hash (copyable)
- Action: `module(function)` format
- Sender (account ID, or "-" if unsigned)
- Nonce
- Fee
- Result: Success / Failed
- Parameters: expandable JSON tree viewer (collapsed by default, expand to 2 levels)
- Signature (or "-" if unsigned)

Tabbed sub-sections:
- **Events tab:** paginated table of events emitted by this extrinsic

---

## Screen 6: Substrate Event List

**Route:** `/sub/event`

Paginated table of runtime events.

Table columns:
- Event index (links to detail)
- Block number (links to block detail)
- Extrinsic index (links to extrinsic detail)
- Action (module + event)
- Timestamp

Pagination: cursor-based.

---

## Screen 7: Substrate Event Detail

**Route:** `/sub/event/[id]`

**Lookup:** by event index.

Event info fields:
- Timestamp
- Blocktime (relative)
- Block number (links to block detail)
- Action: `module(event)` format
- Parameters: expandable JSON tree viewer

---

## Screen 8: Substrate Account List

**Route:** `/sub/account`

Paginated table of Substrate accounts.

Table columns:
- Address (links to detail)
- Balance
- Locked
- Reserved
- Nonce

Pagination: cursor-based.

---

## Screen 9: Substrate Account Detail

**Route:** `/sub/account/[id]`

**Lookup:** by SS58 address.

Account info fields:
- Total Balance (formatted with token decimals and symbol)
- Transferrable (calculated: balance - max(locked, reserved) or balance - locked - reserved depending on `enabledNewTransferableFormulas` flag; floor at 0)
- Locked
- Reserved

Tabbed sub-sections:
- **Extrinsics tab:** paginated table of extrinsics sent by this account
- **Transfers tab:** paginated table of token transfers involving this account

---

## Screen 10: Substrate Transfer List

**Route:** `/sub/transfer`

Paginated table of native token transfers.

Table columns:
- Extrinsic index (links to extrinsic detail)
- Block number
- Timestamp
- Sender address (links to account detail)
- Receiver address (links to account detail)
- Amount (formatted with token decimals and symbol)

Pagination: cursor-based.

---

## Screen 11: EVM Block List

**Route:** `/block`

Paginated table of EVM blocks.

Table columns:
- Block number (links to block detail)
- Timestamp
- Miner address (links to account detail)
- Transaction count

Pagination: cursor-based.

---

## Screen 12: EVM Block Detail

**Route:** `/block/[id]`

**Lookup:** by block number.

Block info fields:
- Block Hash
- Timestamp
- Status: "Finalized"
- Mined by: miner address (links to `/address/[miner]`)
- Transaction count
- Size (in bytes)
- Gas Used
- Gas Limit

Tabbed sub-sections:
- **Transaction tab:** paginated table of transactions in this block

---

## Screen 13: EVM Transaction List

**Route:** `/tx`

Paginated table of EVM transactions.

Table columns:
- Tx hash (links to detail)
- Block number (links to block detail)
- Timestamp
- From address (links to account detail)
- To address (links to account detail)
- Value (formatted with 18 decimals)

Pagination: cursor-based.

---

## Screen 14: EVM Transaction Detail

**Route:** `/tx/[id]`

**Lookup:** by transaction hash.

Transaction info fields:
- Timestamp
- Blocktime (relative)
- Block number (links to `/block/[num]`)
- Status: "Confirmed"
- Tx Hash
- From address (links to `/address/[from]`)
- To address (links to `/address/[to]`)
- Value (formatted with 18 decimals)
- Result: Success / Failed
- Nonce
- Input Data (scrollable, truncated with overflow handling)
- Txn Fee (calculated: gas_used * effective_gas_price for EIP-1559, or gas_used * gas_price for legacy; formatted with 18 decimals)
- Signature (reconstructed from r + s + v fields as single hex string, or "-" if missing)

---

## Screen 15: EVM Account List

**Route:** `/address`

Paginated table of EVM accounts.

Table columns:
- Address (links to detail)
- Balance (formatted with token decimals)

Pagination: cursor-based.

---

## Screen 16: EVM Account Detail

**Route:** `/address/[id]`

**Lookup:** by 0x address.

Account info fields:
- Balance (formatted with token decimals and symbol)

Tabbed sub-sections:
- **ERC-20 Tokens tab:** list of ERC-20 tokens held by this account (name, symbol, balance, contract address)
- **ERC-721 Tokens tab:** list of ERC-721 tokens held by this account (name, symbol, balance, contract address)
- **Transactions tab:** paginated table of transactions involving this address
- **Transfers tab:** paginated table of token transfers involving this address

---

## Screen 17: Smart Contract List

**Route:** `/contract`

Paginated table of deployed smart contracts.

Table columns:
- Contract address (links to detail)
- Contract name
- Transaction count
- Verification status

Pagination: cursor-based.

---

## Screen 18: Smart Contract Detail

**Route:** `/contract/[id]`

**Lookup:** by contract address.

Contract info fields:
- Contract Name
- Creator address (links to `/address/[deployer]`)
- Created At: deployment tx hash (links to `/tx/[hash]`)
- Balance (formatted with token decimals and symbol)

Tabbed sub-sections:
- **Contract tab:**
  - If **verified**: shows contract information panel with:
    - Contract Name, Compiler version, EVM Version, Optimization (with runs count), Revive Version (if applicable)
    - Contract ABI (expandable JSON tree viewer)
    - Contract Source Code (full text, scrollable)
    - Contract Byte Code (full text, scrollable)
  - If **unverified**: shows contract verification form (see Screen 19)
- **Transactions tab:** paginated table of transactions involving this contract
- **ERC-20 Transfers tab:** paginated table of ERC-20 token transfers from/to this contract
- **ERC-721 Transfers tab:** paginated table of ERC-721 token transfers from/to this contract

---

## Screen 19: Contract Verification Form

**Embedded in:** Smart Contract Detail page (Contract tab, when unverified).

Allows the contract deployer to verify source code against deployed bytecode.

### Form Fields

**Compiler Type** (radio): choose between:
- Solidity (Standard-JSON-Input)
- Solidity (Single file)

**Contract Name** (text input, optional)

**Include nightly builds** (radio: Yes/No): controls which compiler versions appear in the dropdown.

**Compiler Version** (dropdown): populated dynamically from API (`/api/plugin/evm/contract/solcs`), filtered by nightly flag.

**Resolc Version** (dropdown): populated dynamically from API (`/api/plugin/evm/contract/resolcs`).

When **Single file** mode:
- **EVM Version** (dropdown): default, london, istanbul, petersburg, constantinople, byzantium, spuriousDragon, tangerineWhistle, homestead
- **Optimization** (radio: Yes/No)
- **Optimization runs** (text input, default 200, shown only when optimization = Yes)
- **Source Code** (multiline textarea, min 10 rows)
- **Compilation Target** (text input, optional, e.g. `myFile.sol`)

When **Standard-JSON-Input** mode:
- **File upload** (single .json file)

### Actions

- **Verify & Publish**: submits to `/api/plugin/evm/etherscan?module=contract&action=verifysourcecode`. On success: page reloads to show verified contract info. On error: shows toast notification with error message.
- **Reset**: reloads the page.
- **Verify Guide**: external link to documentation.

---

## Screen 20: ERC-20 Token List

**Route:** `/erc20_token`

Paginated table of ERC-20 tokens.

Table columns:
- Token name (links to token detail)
- Symbol
- Contract address (links to contract detail)
- Total supply (formatted with token decimals)
- Holders count
- Transfer count

Pagination: cursor-based.

---

## Screen 21: ERC-721 Token List

**Route:** `/erc721_token`

Paginated table of ERC-721 (NFT) tokens.

Table columns:
- Token name (links to token detail)
- Symbol
- Contract address (links to contract detail)
- Total supply
- Holders count
- Transfer count

Pagination: cursor-based.

---

## Screen 22: Token Detail

**Route:** `/token/[id]`

**Lookup:** by contract address. Works for both ERC-20 and ERC-721 tokens.

Token info fields:
- Token Name
- Token Symbol
- Total Supply (formatted with token-specific decimals)
- Decimals
- Category label adapts: "ERC-20 Token" or "ERC-721 Token"

Tabbed sub-sections:
- **Holders tab:** paginated table showing:
  - Holder address (links to `/address/[holder]`)
  - Balance (formatted with token decimals)
- **Transactions tab:** paginated table of transactions involving this token contract
- **Transfers tab:** paginated table of token transfer events showing:
  - Tx hash (links to `/tx/[hash]`)
  - From address
  - To address
  - Value (formatted with token decimals)
  - Timestamp

---

## Shared Behaviors

### Pagination

All list tables use cursor-based pagination with:
- Previous / Next navigation
- Cursor parameters: `after`, `before`, `start_cursor`, `end_cursor`
- Boolean flags: `has_next_page`, `has_previous_page`
- Configurable rows per page (default: 10)

### Data Formatting

- Balances: raw integer values divided by 10^decimals, displayed with comma grouping
- Timestamps: displayed as UTC datetime string
- Relative time: "X minutes ago", "X hours ago" format
- Addresses: full hex shown, linked to their respective detail pages
- Hashes: full hex shown, with copy-to-clipboard where noted
- Large text fields (input data, source code, bytecode): scrollable container with overflow handling

### Loading States

- Full-page spinner during initial metadata/token load
- Inline spinners for individual data fetches
- "No data" text state when API returns empty or null results

### Cross-linking

All entity references are hyperlinked:
- Block numbers link to their respective block detail (Substrate or EVM)
- Addresses link to their respective account detail (Substrate or EVM)
- Transaction hashes link to transaction detail
- Extrinsic indexes link to extrinsic detail
- Contract addresses link to contract detail
- Token names link to token detail

### API Structure

All APIs use POST method with JSON body. Response envelope:

```
{
  code: number,       // 0 = success
  data: T,            // payload
  generated_at: number,
  message: string
}
```

Substrate APIs: `/api/scan/*`, `/api/plugin/balance/*`
EVM APIs: `/api/plugin/evm/*`
