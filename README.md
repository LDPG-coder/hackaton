# Ven&Pay — Pay-Per-Call MCP Wallet & Verified Marketplace on Solana

> **Stop paying \$20–\$99/month × 10 SaaS subscriptions just to let your AI agent call external tools.**  
> **Ven&Pay** (*Venezuela & Pay* / *"Ven y Paga"*) brings **`x402` Solana micropayments**, **on-chain spending guardrails**, and **verified MCP tools** directly into **Antigravity, OpenCode, Cursor, and Claude Code** — with Venezuelan top-up rails via **Coco Wallet** and **Pollar**.

Built for the **Dev3Pack Global Hackathon (Nov 27–29, 2026)** · Hub: **Build Station Caracas 🇻🇪**

---

## Solana Deployment (Devnet)

| Artifact | Value |
|---|---|
| **Network** | Solana Devnet |
| **Program ID (`venpay`)** | `TBD — deploying in Stage 1 via Anchor / Solana Playground` |
| **Explorer** | `TBD` |
| **Live Web App & Sandbox** | `TBD — deploying on Vercel` |
| **Demo Video (≤ 2 min)** | `TBD` |

> **Transparency Note (Hackathon Rule):** 100% of this project is built from scratch during the Dev3Pack Global Hackathon (Nov 27–29, 2026).

---

## Why Ven&Pay?

1. **The Problem (Especially Acute in Venezuela & LATAM):**  
   AI coding harnesses (**Antigravity**, **OpenCode**, **Cursor**, **Claude Code**) become dramatically more powerful with external MCP tools (web scraping, voice synthesis, design, on-chain data). Today, every provider demands a separate monthly subscription and an international credit card — or forces devs to trust unverified third-party MCP servers.
2. **The Solution:**  
   - **On-Chain Verified MCP Registry & Agent Vault (Solana Rust/Anchor):** Tools are registered on-chain with verifiable pricing and call/reliability counters. Users fund a non-custodial **Agent Vault PDA** with strict `max_per_call` limits and 1-click withdrawal — an agent can never drain your main wallet.
   - **Universal Harness Plugin (`packages/mcp-plugin`):** One lightweight `stdio` MCP bridge for **Antigravity**, **OpenCode**, **Cursor**, and **Claude Code** that transparently negotiates `HTTP 402 Payment Required` (`x402`), settles micropayments on Solana Devnet, and returns real tool results with **zero SaaS API keys required from the end user**.
   - **Curated Real Tools Live Today:** Powered by real APIs (**Firecrawl** web extraction, **ElevenLabs** voice synthesis, and **Solana On-Chain Audit**).
   - **Local Venezuelan Rails:** Onboarding & wallet management via **Pollar**, plus a companion **Coco Wallet Mini App** (`@cocowallet/miniapp-sdk`) to top up credits in **USDC / Pago Móvil (Bs)** and receive push spend alerts.

---

## Architecture Overview

```text
Antigravity / OpenCode / Cursor / Claude Code
          │  (MCP stdio / HTTP)
          ▼
 packages/mcp-plugin  ───────►  Solana Devnet (Anchor Program: venpay)
 (x402 Wallet Proxy)              ├── ToolRegistry PDA (Verified Tools + Reputation)
          │                       ├── AgentVault PDA   (Prepaid Cap + max_per_call)
          │ (HTTP 402 / X-PAYMENT)└── settle_call      (Atomic 95/5 Split + Event)
          ▼
   apps/web (Next.js) ───────►  Real Upstream Tool Providers
   ├── x402 MCP Gateway           ├── Firecrawl (Web Scraping & Extraction)
   ├── Marketplace & Analytics    ├── ElevenLabs (Voice Synthesis TTS)
   ├── Pollar Wallet Onboarding   └── Solana RPC (On-Chain Account Audit)
   └── 1-Click Live Sandbox
          ▲
          │ (Receipts, Analytics, Anti-Replay Nonces, Top-Up Ledger)
       Supabase  ◄───────────  apps/coco-miniapp (Coco Wallet WebView: USDC / Pago Móvil Bs)
```

For full technical architecture, PDA schemas, design patterns, and the `x402` wire contract, see [PROJECT.md](./PROJECT.md).

---

## Monorepo Structure

```text
hackaton/
├── README.md                 # Project overview, deployment addresses, and quickstart
├── PROJECT.md                # Full technical spec, design patterns, schemas & stages
├── AGENTS.md                 # Fast-cycle engineering rules for Antigravity & OpenCode
├── programs/
│   └── venpay/               # Rust/Anchor Solana program (ToolRegistry + AgentVault)
├── packages/
│   ├── shared/               # Shared types, Zod schemas, Anchor IDL, PaymentRail interfaces
│   └── mcp-plugin/           # Universal stdio MCP + x402 wallet bridge
├── plugins/
│   ├── antigravity/          # Native Antigravity plugin bundle (plugin.json + mcp_config.json + SKILL.md)
│   ├── opencode/             # Ready-to-use opencode.json (Desktop & CLI)
│   └── claude-cursor/        # Standard .mcp.json for Claude Code & Cursor
└── apps/
    ├── web/                  # Next.js Marketplace, x402 Gateway, Analytics, Pollar & Live Sandbox
    └── coco-miniapp/         # Coco Wallet Mini App (Bs/USDC top-up & push notifications)
```

---

## Dev3Pack Hackathon Tracks

- **Solana — Best App Overall & Best Agentic App:** Custom Rust/Anchor program on Devnet + `x402` machine micropayments + on-chain spending guardrails.
- **Dev3pack Venezuela (Local Track):** Solves international SaaS payment barriers for Venezuelan/LATAM developers.
- **Coco Wallet (Venezuela Local Track):** Companion Mini App built with `coco-mini-apps` (`@cocowallet/miniapp-sdk`) for Bs/USDC credit top-ups and spending alerts.
- **Pollar (Payments):** Social login, non-custodial Solana wallet provisioning, and sponsored payments via `@pollar/react`.
- **Firecrawl (AI):** Real pay-per-call `firecrawl_scrape` MCP tool.
- **ElevenLabs (AI):** Real pay-per-call `elevenlabs_tts` MCP tool with live audio output.
- **Supabase:** Verifiable call receipts, savings analytics, anti-replay ledger, and top-up sync.

---

## Quickstart (Local Development)

> Prerequisites: Node.js 20+ (LTS), `pnpm`, Git (macOS & Windows non-admin compatible; always bind dev servers to `127.0.0.1`).

```bash
# 1. Clone repository
git clone https://github.com/LDPG-coder/hackaton.git
cd hackaton

# 2. Install dependencies (once workspace packages are initialized)
pnpm install

# 3. Configure environment variables (never commit .env.local!)
cp apps/web/.env.example apps/web/.env.local

# 4. Run web app & x402 MCP Gateway locally
pnpm --filter web dev -- --hostname 127.0.0.1
```

*(Detailed setup commands for the Antigravity plugin, OpenCode `opencode.json`, Anchor Devnet deploy, and Coco Mini App preview will be updated as each stage lands.)*

---

## Roadmap & Coming Soon

- **Phase 1 (Hackathon MVP — Live):** Solana Devnet Anchor Vault & Registry, x402 Gateway with Firecrawl + ElevenLabs + Solana Audit, Antigravity & OpenCode plugin, Pollar onboarding, Supabase analytics, Live Judge Sandbox, and Coco Wallet Top-Up Mini App.
- **Phase 2 (Coming Soon — Post-Hackathon):**
  - **Open Community MCP Publishing ("Publish Your MCP"):** Self-serve provider onboarding gated by automated security sandboxing, schema validation, and stake-slashing audits (intentionally deferred from the 36-hour sprint so only verified, safe tools are exposed in v1).
  - **Mainnet USDC Settlement & Cross-Chain Auto-Bridging:** Production SPL USDC settlement with automatic Polygon/Coco -> Solana liquidity routing.
  - **Expanded SaaS Catalog:** Figma, Linear, Notion, and GitHub Pro pay-per-call connectors.

---

## License

MIT
