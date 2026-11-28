// Paste into Solana Playground (beta.solpg.io) -> tests/anchor.test.ts
// Globals injected by Solana Playground: pg, web3, anchor, BN, assert

describe("Ven&Pay Devnet Protocol & Vault Guardrails", () => {
  const owner = pg.wallet.publicKey;
  const treasury = owner;
  const provider = owner;
  const feeBps = 500; // 5%

  const tools = [
    { slug: "firecrawl_scrape", priceLamports: 2_000_000 }, // 0.002 SOL
    { slug: "elevenlabs_tts", priceLamports: 5_000_000 }, // 0.005 SOL
    { slug: "solana_audit", priceLamports: 1_000_000 }, // 0.001 SOL
  ];

  const [configPda] = web3.PublicKey.findProgramAddressSync(
    [Buffer.from("config")],
    pg.program.programId
  );

  const [vaultPda] = web3.PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), owner.toBuffer()],
    pg.program.programId
  );

  const getToolPda = (slug: string) =>
    web3.PublicKey.findProgramAddressSync(
      [Buffer.from("tool"), Buffer.from(slug)],
      pg.program.programId
    )[0];

  // Deterministic 32-byte schema hash derived from tool slug for Devnet seeding
  const makeSchemaHash = (slug: string): number[] => {
    const hash = new Array<number>(32).fill(0);
    const bytes = Buffer.from(slug, "utf8");
    for (let i = 0; i < Math.min(bytes.length, 32); i++) {
      hash[i] = bytes[i];
    }
    return hash;
  };

  const delegateKp = web3.Keypair.generate();

  it("initializes protocol config and registers curated MCP tools (idempotent)", async () => {
    const existingConfig = await pg.program.account.protocolConfig.fetchNullable(
      configPda
    );

    if (!existingConfig) {
      const tx = await pg.program.methods
        .initializeProtocol(treasury, feeBps)
        .accounts({
          admin: owner,
          config: configPda,
          systemProgram: web3.SystemProgram.programId,
        })
        .rpc();
      await pg.connection.confirmTransaction(tx, "confirmed");
      console.log("Protocol initialized:", tx);
    }

    const config = await pg.program.account.protocolConfig.fetch(configPda);
    assert.equal(config.feeBps, feeBps);

    for (const item of tools) {
      const toolPda = getToolPda(item.slug);
      const existingTool = await pg.program.account.toolRegistry.fetchNullable(
        toolPda
      );

      if (!existingTool) {
        const tx = await pg.program.methods
          .registerTool(
            item.slug,
            provider,
            new BN(item.priceLamports),
            makeSchemaHash(item.slug)
          )
          .accounts({
            admin: owner,
            config: configPda,
            tool: toolPda,
            systemProgram: web3.SystemProgram.programId,
          })
          .rpc();
        await pg.connection.confirmTransaction(tx, "confirmed");
        console.log(`Registered tool ${item.slug}:`, tx);
      }

      const registered = await pg.program.account.toolRegistry.fetch(toolPda);
      assert.equal(registered.slug, item.slug);
      assert(registered.verified);
      assert(registered.active);
    }
  });

  it("creates/updates vault, funds delegate & vault, settles call as delegate, and enforces caps", async () => {
    const maxPerCall = new BN(5_000_000); // 0.005 SOL cap
    const existingVault = await pg.program.account.agentVault.fetchNullable(
      vaultPda
    );

    if (!existingVault) {
      const createTx = await pg.program.methods
        .createVault(delegateKp.publicKey, maxPerCall)
        .accounts({
          owner,
          vault: vaultPda,
          systemProgram: web3.SystemProgram.programId,
        })
        .rpc();
      await pg.connection.confirmTransaction(createTx, "confirmed");
      console.log("Vault created:", createTx);
    } else {
      const updateTx = await pg.program.methods
        .updateVault(delegateKp.publicKey, maxPerCall, true)
        .accounts({
          owner,
          vault: vaultPda,
        })
        .rpc();
      await pg.connection.confirmTransaction(updateTx, "confirmed");
    }

    // Fund delegate slightly so it can pay Devnet tx fees when calling settle_call directly
    const fundDelegateTx = new web3.Transaction().add(
      web3.SystemProgram.transfer({
        fromPubkey: owner,
        toPubkey: delegateKp.publicKey,
        lamports: 3_000_000, // 0.003 SOL for test tx fees
      })
    );
    const fundSig = await pg.program.provider.sendAndConfirm(fundDelegateTx);
    console.log("Funded test delegate:", fundSig);

    // Deposit 0.02 SOL into AgentVault
    const depositAmount = new BN(20_000_000);
    const vaultBeforeDeposit = await pg.program.account.agentVault.fetch(
      vaultPda
    );
    const depositTx = await pg.program.methods
      .depositVault(depositAmount)
      .accounts({
        depositor: owner,
        vault: vaultPda,
        systemProgram: web3.SystemProgram.programId,
      })
      .rpc();
    await pg.connection.confirmTransaction(depositTx, "confirmed");

    const vaultAfterDeposit = await pg.program.account.agentVault.fetch(
      vaultPda
    );
    assert(
      vaultAfterDeposit.balanceLamports.eq(
        vaultBeforeDeposit.balanceLamports.add(depositAmount)
      )
    );

    // Settle a firecrawl_scrape call signed ONLY by the scoped delegate keypair
    const scrapeToolPda = getToolPda("firecrawl_scrape");
    const toolBefore = await pg.program.account.toolRegistry.fetch(
      scrapeToolPda
    );
    const nonce = `vp_test_${Date.now()}`;

    const settleTx = await pg.program.methods
      .settleCall(nonce)
      .accounts({
        authority: delegateKp.publicKey,
        config: configPda,
        tool: scrapeToolPda,
        vault: vaultPda,
        provider,
        treasury,
      })
      .signers([delegateKp])
      .rpc();
    await pg.connection.confirmTransaction(settleTx, "confirmed");
    console.log("Scoped delegate settle_call tx:", settleTx);

    const vaultAfterSettle = await pg.program.account.agentVault.fetch(
      vaultPda
    );
    const toolAfterSettle = await pg.program.account.toolRegistry.fetch(
      scrapeToolPda
    );
    assert(
      vaultAfterSettle.balanceLamports.eq(
        vaultAfterDeposit.balanceLamports.sub(toolBefore.priceLamports)
      )
    );
    assert(
      toolAfterSettle.totalCalls.eq(toolBefore.totalCalls.add(new BN(1)))
    );

    // Negative Security Check 1: delegate MUST NOT be able to withdraw_vault
    let withdrawBlocked = false;
    try {
      await pg.program.methods
        .withdrawVault(new BN(1_000_000))
        .accounts({
          owner: delegateKp.publicKey,
          vault: vaultPda,
        })
        .signers([delegateKp])
        .rpc();
    } catch (_err) {
      withdrawBlocked = true;
    }
    assert(withdrawBlocked, "Delegate should never be allowed to withdraw");

    // Negative Security Check 2: settle_call MUST fail if tool price > vault.max_per_call
    const lowerCapTx = await pg.program.methods
      .updateVault(delegateKp.publicKey, new BN(500_000), true)
      .accounts({
        owner,
        vault: vaultPda,
      })
      .rpc();
    await pg.connection.confirmTransaction(lowerCapTx, "confirmed");

    let capBlocked = false;
    try {
      await pg.program.methods
        .settleCall(`vp_cap_${Date.now()}`)
        .accounts({
          authority: delegateKp.publicKey,
          config: configPda,
          tool: scrapeToolPda,
          vault: vaultPda,
          provider,
          treasury,
        })
        .signers([delegateKp])
        .rpc();
    } catch (_err) {
      capBlocked = true;
    }
    assert(capBlocked, "settle_call exceeding max_per_call should fail");

    // Restore cap and test 100% owner withdrawal (verifying rent-exemption preservation)
    const restoreTx = await pg.program.methods
      .updateVault(delegateKp.publicKey, maxPerCall, true)
      .accounts({
        owner,
        vault: vaultPda,
      })
      .rpc();
    await pg.connection.confirmTransaction(restoreTx, "confirmed");

    const vaultBeforeWithdraw = await pg.program.account.agentVault.fetch(
      vaultPda
    );
    const withdrawTx = await pg.program.methods
      .withdrawVault(vaultBeforeWithdraw.balanceLamports)
      .accounts({
        owner,
        vault: vaultPda,
      })
      .rpc();
    await pg.connection.confirmTransaction(withdrawTx, "confirmed");

    const vaultFinal = await pg.program.account.agentVault.fetch(vaultPda);
    assert(vaultFinal.balanceLamports.isZero());

    console.log("=== VENPAY DEVNET VERIFICATION SUMMARY ===");
    console.log("Program ID:", pg.program.programId.toBase58());
    console.log("ProtocolConfig PDA:", configPda.toBase58());
    console.log("AgentVault PDA:", vaultPda.toBase58());
    console.log(
      "Explorer:",
      `https://explorer.solana.com/address/${pg.program.programId.toBase58()}?cluster=devnet`
    );
  });
});
