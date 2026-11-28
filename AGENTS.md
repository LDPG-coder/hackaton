# Ven&Pay — Agent & Engineering Rules (`AGENTS.md`)

Fast-cycle engineering guardrails for **Ven&Pay** (Dev3Pack Global Hackathon). Applies to **Antigravity**, **OpenCode**, and all contributors working in this repository.

## 1. Dual-OS Environment & Portability (macOS Dev + Windows `FCMICROS-22` Parity)
- **macOS (`arm64` Dev Machine):** Native `git`, `node 22`, `pnpm 10`, `opencode`, and `docker` are available. Always quote local workspace paths containing special characters (e.g., `"/Users/Porto/Desktop/Ven&pay"`).
- **Windows (`FCMICROS-22` Non-Admin Hackathon Machine):** No Admin, no WSL2, no Docker, `LongPathsEnabled=0`, Windows Firewall UAC prompts on `0.0.0.0`, shared PC with `VeyonService`. Use user-space tooling only (`scoop`, `pnpm`, `uv`, `curl.exe`).
- **Rust / Solana Builds:** Compile and deploy `programs/venpay` via **Solana Playground (`beta.solpg.io`)** (or GitHub Codespaces); keep the exact `programs/venpay/src/lib.rs` source and exported `packages/shared/idl/venpay.json` committed in this repository so neither macOS nor Windows requires a local Rust/Anchor toolchain.
- **Local Dev Servers:** Always bind to **`127.0.0.1` / `localhost`** (never `0.0.0.0`) to avoid Windows Firewall UAC prompts.
- **Cross-Platform Scripts & Paths:** All `package.json` scripts MUST use Node/`pnpm`/`tsx` (never bash-only or PowerShell-only shell syntax). Keep paths concise, avoid creating symlinks (NTFS compatibility), enforce `LF` line endings via `.gitattributes`, and NEVER hardcode OS-specific absolute paths (`C:\Users\...` or `/Users/...`) in `plugins/` manifests or committed config files.
- **Handoff Hygiene:** Keep `.env.example` up to date with every required variable for zero-friction OS switching (`git pull` + copy `.env.local` + `pnpm install`), and run `gh auth logout` on shared Windows PCs after the hackathon.

## 2. Security & Shared-PC Hygiene (NON-NEGOTIABLE)
- **NEVER commit secrets:** Never commit `.env`, `.env.local`, API keys (Firecrawl, ElevenLabs, Supabase service role, Pollar secret, Gemini/OpenAI), or Solana keypair JSON files. Verify `git status` and `.gitignore` before every push.
- **NEVER ask users for SaaS API keys or main wallet private keys:** End users pay per call via Solana; upstream provider API keys stay strictly server-side in `apps/web` environment variables.
- **Scoped Delegate Keys Only:** The local harness plugin (`packages/mcp-plugin`) may only sign `settle_call` transactions using a scoped `delegate` keypair constrained by `AgentVault.max_per_call` on-chain. Only the vault `owner` can call `withdraw_vault`.
- **Mandatory `x402` Anti-Replay:** Every paid gateway endpoint MUST verify: (a) unexpired single-use `nonce`, (b) confirmed Solana Devnet tx matching `VENPAY_PROGRAM_ID`, `toolSlug`, `nonce`, and `amountLamports`, and (c) unique `tx_signature` not previously redeemed.
- **Sanitize External Tool Inputs:** Validate all tool payloads with `Zod`, enforce URL `http/https` allowlists (block `localhost`/private IPs to prevent SSRF in Firecrawl calls), and cap ElevenLabs TTS input length (max 300 chars in demo mode).

## 3. Architecture & Design Pattern Rules
- **Respect Layer Boundaries:**
  - `packages/shared` -> Pure TypeScript types, Zod schemas, Anchor IDL, and adapter interfaces (zero Next.js or DOM imports).
  - `packages/mcp-plugin` -> Strictly **stdio-clean** (`console.error` ONLY for diagnostics; NEVER write logs to `stdout` or MCP JSON-RPC framing will break!).
  - `apps/web` -> Implement all payment rails behind `PaymentRailAdapter`, all upstream MCP tools behind `McpToolProvider`, all `x402` checks in the middleware pipeline, and all Supabase calls inside `lib/repositories/`.
  - `apps/coco-miniapp` -> Follow Coco's dark UI kit (`#181818`, `#00AF71`, Poppins, Spanish copy) and guard every `@cocowallet/miniapp-sdk` call with `window.flutter_inappwebview` checks so it never crashes outside the mobile WebView.
- **Keep Scope Locked:** Third-party open MCP publishing (`+ Publish Your MCP`) is intentionally **Coming Soon** in v1. Do not expand scope before the core end-to-end demo works flawlessly.

## 4. Fast-Cycle Coding & Verification Discipline
- **Vertical Slices First:** Always wire end-to-end functionality (Solana tx -> Gateway verify -> Real provider -> UI receipt) before cosmetic polish.
- **No Unverified Claims or Mocked Integrations:** Never claim a partner integration (Solana Anchor, Pollar, Coco Wallet, Firecrawl, ElevenLabs, Supabase) is active unless real working code executes it.
- **Verify After Every Milestone:** Run `pnpm typecheck` / `pnpm build` (and the `x402` smoke test when touching gateway/plugin code) before committing.
- **Commit Cleanly:** Small, atomic commits on `main` (or short feature branches) with clear messages so work is continuously backed up to GitHub.

## 5. Hackathon Compliance Guardrails (Dev3Pack)
- **Solana Core Mandatory:** All core registry & settlement logic MUST run on **Solana Devnet** with a unique Rust/Anchor program; Program ID MUST appear in `README.md` and Step 1 of submission.
- **Coco Wallet Boundary:** Coco Mini App runs on Polygon (`137`) / Pago Móvil inside Coco Wallet WebView as the **top-up & alerts companion**, never as a replacement for the Solana smart contract.
- **Demo Video Limit:** Keep the final demo video **strictly ≤ 2:00 minutes** (satisfies both Solana `< 3:00` and Pollar `≤ 2:00` disqualification rules), showing live product execution, not slides.
