# Ven&Pay — Technical Specification (`PROJECT.md`)

This document is the single source of truth for **Ven&Pay**'s technical architecture, design patterns, on-chain schemas, `x402` wire protocol, integrations, and staged delivery plan.

---

## 1. Product Scope & Boundaries

### In Scope (Hackathon MVP — 33 Hours)
1. **Solana Devnet Smart Contract (`programs/venpay`):** Unique Rust/Anchor program managing verified MCP tool pricing/reputation and user spending vaults with per-call caps.
2. **x402 MCP Gateway + Curated Real Providers (`apps/web`):** HTTP `402 Payment Required` gateway wrapping real upstream APIs:
   - `firecrawl_scrape` (**Firecrawl**) — URL scraping & clean markdown/JSON extraction.
   - `elevenlabs_tts` (**ElevenLabs**) — Text-to-speech audio generation.
   - `solana_audit` (**Solana Devnet/Mainnet RPC**) — Read-only Solana wallet/token inspection.
3. **Universal Harness Plugin (`packages/mcp-plugin` + `plugins/`):** Dual-transport (`stdio` local + Streamable HTTP remote) MCP Wallet Proxy tested directly in **Antigravity Desktop** and **OpenCode Desktop** (and compatible with their CLIs, Cursor, and Claude Code).
4. **Web Platform (`apps/web` on Vercel):**
   - **Curated Verified Marketplace** (live real tools + *"Community MCP Publishing & Automated Security Audits — Coming Soon"* cards).
   - **Agent Vault Dashboard** (deposit/withdraw Devnet SOL, set `max_per_call`, assign scoped `delegate` session key, copy harness configs).
   - **Pollar Integration** (`@pollar/react`, `@pollar/core`) for social login & Solana wallet onboarding.
   - **Supabase Analytics & Receipts** (spend breakdown, savings vs. monthly SaaS subscriptions, clickable Solana Explorer links).
   - **1-Click Live Devnet Sandbox** for zero-friction judge testing.
5. **Coco Wallet Mini App (`apps/coco-miniapp`):** Venezuelan mobile companion built with `coco-mini-apps` (`@cocowallet/miniapp-sdk`) for **USDC / Pago Móvil (Bs)** credit top-ups and push spending alerts.

### Explicitly Out of Scope for v1 (Marked "Coming Soon")
- Open, unvetted third-party MCP publishing (`+ Publish Your MCP`), because production safety requires automated endpoint sandboxing, SSRF isolation, and economic slashing.
- Mainnet real-money bridging between Polygon (Coco Wallet) and Solana (handled via a transparent Devnet credit-bridge ledger in v1, documented clearly for judges).

---

## 2. Harness Targeting Strategy (CLI vs. Desktop vs. Web)

- **Primary Test Harnesses:** **Antigravity Desktop** and **OpenCode Desktop** (both installed locally).
- **Why Local `stdio` + Remote HTTP Works Everywhere:**
  - **Antigravity Desktop & CLI** share the exact same plugin layout (`plugin.json` + `mcp_config.json` + `skills/venpay/SKILL.md`).
  - **OpenCode Desktop & CLI** share the exact same `opencode.json` schema (`type: "local"` for `stdio`, `type: "remote"` for HTTP).
  - Running the MCP Wallet Proxy locally via **`stdio`** keeps the scoped Solana session signer on the user's machine (never uploading private keys to a remote server).
  - Exposing **`/api/mcp`** (Streamable HTTP) alongside `/api/x402/tools/[slug]` also supports web clients and our zero-install **Live Sandbox**.

---

## 3. Design Patterns & Clean Boundaries

| Layer | Pattern | Contract / Implementation |
|---|---|---|
| **Payments & Wallets** | **Adapter / Strategy** | `PaymentRailAdapter` implemented by `SolanaVaultAdapter`, `PollarWalletAdapter`, `DevnetBurnerAdapter` (sandbox), and `CocoMiniAppAdapter`. |
| **x402 Gateway** | **Chain of Responsibility** | Ordered pipeline: `ValidateInput` -> `LookupVerifiedTool` -> `RequireX402Challenge` -> `VerifySolanaTxAndNonce` -> `ExecuteProvider` -> `PersistReceipt`. |
| **MCP Tool Backends** | **Factory + Strategy** | `McpToolProvider` interface implemented by `FirecrawlProvider`, `ElevenLabsProvider`, and `SolanaAuditProvider`, registered in `ToolProviderRegistry`. |
| **Persistence** | **Repository** | `ReceiptRepository`, `NonceRepository`, `TopUpRepository` wrapping Supabase with strict TypeScript DTOs and an in-memory fallback for unit tests. |
| **Harness Integration** | **Proxy / Facade** | `VenPayMcpProxy` (`packages/mcp-plugin`) translates standard MCP tool calls into `402` challenge negotiation + Solana `settle_call` signing + retry. |

---

## 4. Solana Smart Contract Specification (`programs/venpay`)

- **Framework:** Anchor (Rust), compiled & deployed to **Solana Devnet** via Solana Playground (`beta.solpg.io`) or GitHub Codespaces.

### On-Chain Accounts (PDAs)
1. **`ProtocolConfig`** (`seeds = [b"config"]`)
   - `admin: Pubkey`
   - `treasury: Pubkey`
   - `fee_bps: u16` (default `500` = 5%)
   - `total_calls: u64`
   - `total_volume_lamports: u64`
   - `bump: u8`
2. **`ToolRegistry`** (`seeds = [b"tool", slug.as_bytes()]`)
   - `slug: String` (max 32 bytes, e.g. `"firecrawl_scrape"`)
   - `provider: Pubkey` (payout wallet)
   - `price_lamports: u64` (e.g. `2_000_000` = 0.002 SOL)
   - `schema_hash: [u8; 32]` (verifiable SHA-256 of tool spec)
   - `verified: bool`
   - `active: bool`
   - `total_calls: u64`
   - `bump: u8`
3. **`AgentVault`** (`seeds = [b"vault", owner.key().as_ref()]`)
   - `owner: Pubkey` (user's main wallet)
   - `delegate: Pubkey` (scoped session keypair used by Antigravity/OpenCode plugin; can ONLY call `settle_call`, NEVER `withdraw_vault`)
   - `balance_lamports: u64`
   - `max_per_call: u64` (hard on-chain spending cap per call)
   - `total_spent: u64`
   - `call_count: u64`
   - `active: bool`
   - `bump: u8`

### Instructions
- `initialize_protocol(fee_bps: u16)` — Sets admin & treasury once.
- `register_tool(slug: String, price_lamports: u64, schema_hash: [u8; 32])` — Admin registers verified curated tools.
- `create_vault(delegate: Pubkey, max_per_call: u64)` — Initializes user's `AgentVault` PDA.
- `update_vault(delegate: Pubkey, max_per_call: u64, active: bool)` — Owner rotates session key, adjusts cap, or freezes agent spending immediately.
- `deposit_vault(amount: u64)` — Transfers SOL into the `AgentVault` PDA.
- `withdraw_vault(amount: u64)` — **Owner-only** withdrawal back to owner's main wallet (preserving rent-exempt minimum).
- `settle_call(nonce: String)` — Callable by `owner` OR `delegate`:
  1. Checks `vault.active && tool.active && tool.verified`.
  2. Checks `tool.price_lamports <= vault.max_per_call` and `vault.balance_lamports >= tool.price_lamports`.
  3. Atomically transfers `95%` to `tool.provider` and `5%` to `config.treasury` from `AgentVault`.
  4. Increments `tool.total_calls`, `vault.call_count`, `vault.total_spent`, `config.total_calls`, `config.total_volume_lamports`.
  5. Emits `CallSettledEvent { vault, owner, delegate, tool_slug, amount_lamports, nonce, timestamp }`.

---

## 5. `x402` Wire Protocol Contract

### Step 1: Unpaid Request
```http
POST /api/x402/tools/firecrawl_scrape
Content-Type: application/json

{ "url": "https://solana.com" }
```

### Step 2: `HTTP 402 Payment Required` Challenge
```http
HTTP/1.1 402 Payment Required
Content-Type: application/json
X-PAYMENT-REQUIRED: solana-devnet

{
  "protocol": "x402-solana-venpay-v1",
  "network": "solana-devnet",
  "programId": "<VENPAY_PROGRAM_ID>",
  "toolSlug": "firecrawl_scrape",
  "amountLamports": 2000000,
  "estimatedUsd": 0.004,
  "retailMonthlyUsd": 19.00,
  "nonce": "vp_9f3c2a1b...",
  "expiresAt": "2026-11-28T16:05:00Z"
}
```

### Step 3: Paid Retry (`X-PAYMENT`)
```http
POST /api/x402/tools/firecrawl_scrape
Content-Type: application/json
X-PAYMENT: {"signature":"5xK...","nonce":"vp_9f3c2a1b...","vaultOwner":"7zP...","mode":"vault_settle"}

{ "url": "https://solana.com" }
```

### Step 4: On-Chain Verification & Anti-Replay Rules
1. Validate `nonce` exists in `x402_nonces`, is unexpired, and `used_at IS NULL`.
2. Fetch confirmed transaction via Solana Devnet RPC (`getTransaction` with `commitment: "confirmed"`).
3. Verify transaction succeeded (`meta.err === null`), invoked `VENPAY_PROGRAM_ID`, and matches `toolSlug`, `nonce`, and `amountLamports` (from program logs / event).
4. Atomically mark `nonce` + `signature` as consumed in Supabase (`UNIQUE(tx_signature)` prevents replay attacks).
5. Invoke upstream provider (`Firecrawl` / `ElevenLabs` / `SolanaAudit`), store receipt, and return `200 OK` with result + Solana Explorer link.

---

## 6. Supabase Database Schema

- **`x402_nonces`**: `nonce (PK)`, `tool_slug`, `amount_lamports`, `expires_at`, `used_at`, `tx_signature (UNIQUE)`.
- **`call_receipts`**: `id (UUID PK)`, `created_at`, `wallet_address`, `harness` (`antigravity` | `opencode` | `sandbox` | `other`), `tool_slug`, `amount_lamports`, `estimated_usd`, `retail_monthly_usd`, `tx_signature (UNIQUE)`, `status`, `latency_ms`, `cached`.
- **`coco_topups`**: `id (UUID PK)`, `created_at`, `coco_wallet_address`, `solana_vault_owner`, `payment_method` (`polygon_usdc` | `pago_movil_bs`), `amount_usd`, `coco_tx_hash (UNIQUE)`, `devnet_credit_tx`, `status`.

---

## 7. Coco Wallet Mini App Integration (`apps/coco-miniapp`)

- **SDK:** `@cocowallet/miniapp-sdk` (runs inside Coco Wallet mobile WebView on Polygon `chainId 137` + Pago Móvil Bs).
- **Theme Compliance:** Strictly follows Coco's dark UI kit (`#181818` background, `#222222` cards, `#00AF71` green, Poppins, Spanish copy, ≥ 44px touch targets).
- **Capabilities Used:**
  - `MiniApp.getAddress()` & `MiniApp.getUserInfo()`
  - `MiniApp.sendTransaction()` (gasless USDC top-up with `requiredFunds` auto-swap support) & `MiniApp.recordTransaction()`
  - `MiniApp.requestPagoMovilPayout()` option / Bs equivalence display for Venezuelan users
  - `MiniApp.notifications.enable()` + spend threshold alerts
- **Bridge to Solana Core:** Submitting a top-up posts the verified receipt to `apps/web/api/coco/topup`, which records it in Supabase (`coco_topups`) and credits Devnet test SOL to the linked Solana `AgentVault` so the user's Antigravity/OpenCode agent can spend immediately.

---

## 8. Staged Execution Plan

| Stage | Goal | Exit Verification |
|---|---|---|
| **0** | Repo docs (`README.md`, `PROJECT.md`, `AGENTS.md`, `.gitignore`) | Docs committed & pushed to `main` |
| **1** | Deploy `venpay` Rust/Anchor program to Solana Devnet | Verified Program ID on Solana Explorer + IDL exported |
| **2** | Monorepo scaffold + Supabase + x402 Gateway + Firecrawl/ElevenLabs/SolanaAudit | Automated test: `402` -> Devnet tx -> `200` -> replay `409` |
| **3** | `packages/mcp-plugin` + Antigravity & OpenCode manifests | Live tool calls verified inside Antigravity Desktop & OpenCode Desktop |
| **4** | Web Marketplace, Vault Dashboard, Pollar, Analytics & Live Sandbox | Deployed on Vercel; 1-click judge demo runs cleanly |
| **5** | Coco Wallet Mini App (`apps/coco-miniapp`) | Verified via `coco-mini-apps` MCP / Coco Wallet preview |
| **6** | Polish, README Program ID update, ≤ 1:55 min demo video & Dev3Pack submit | All 5 submission steps validated before Sunday 01:00 AM Caracas |
