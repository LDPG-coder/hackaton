# `programs/venpay` — Solana Playground Deployment Runbook

Deploy the `venpay` Anchor smart contract directly to **Solana Devnet** from **macOS** or **Windows** without installing Rust, Solana CLI, or Anchor locally.

---

## 1. Create & Build in Solana Playground
1. Open **[beta.solpg.io](https://beta.solpg.io)** in your browser.
2. Connect or generate a **Devnet Playground Wallet** (bottom-left) and confirm you have at least `~2.5 SOL` on Devnet (use the Playground terminal `solana airdrop 2` or a Devnet faucet such as `faucet.solana.com`).
3. Create a new project named **`venpay`** selecting the **Anchor (Rust)** template.
4. Replace `src/lib.rs` in Solana Playground with the full contents of [`src/lib.rs`](./src/lib.rs).
5. Click **Build** (wrench icon). On the first build, Solana Playground generates a Program ID keypair; if the console notes a `declare_id!` mismatch, update `declare_id!("...")` inside Playground (and in [`src/lib.rs`](./src/lib.rs)) and click **Build** again.

## 2. Deploy & Verify on Devnet
1. In the **Build & Deploy** panel, click **Deploy** (verify cluster is **Devnet**).
2. Copy the resulting **Program ID**.
3. Replace `tests/anchor.test.ts` in Solana Playground with [`tests/venpay.solpg.ts`](./tests/venpay.solpg.ts) and click **Test** — this initializes `ProtocolConfig`, registers `firecrawl_scrape`, `elevenlabs_tts`, and `solana_audit`, tests delegate-signed `settle_call`, enforces `max_per_call` & owner-only withdraw, and prints the Explorer link.

## 3. Export Artifacts Back Into This Repository
After a clean Devnet deploy and test run:
1. Export the **IDL JSON** from Solana Playground (**Build & Deploy -> IDL -> Export** or copy from `target/idl/venpay.json`) and save it to `packages/shared/idl/venpay.json`.
2. Update `declare_id!("<PROGRAM_ID>")` in [`src/lib.rs`](./src/lib.rs) and `[programs.devnet]` in [`Anchor.toml`](./Anchor.toml).
3. Update the **Program ID** and **Explorer** URL at the top of the root [`README.md`](../../README.md).
