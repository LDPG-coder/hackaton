use anchor_lang::prelude::*;
use anchor_lang::system_program;

// Placeholder ID; Solana Playground (beta.solpg.io) generates and syncs your
// deployed Devnet Program ID here on Build/Deploy. Commit the updated ID afterward.
declare_id!("11111111111111111111111111111111");

pub const MAX_SLUG_LEN: usize = 32;
pub const MAX_NONCE_LEN: usize = 64;
pub const MAX_FEE_BPS: u16 = 2_000; // 20% hard ceiling; default protocol fee is 500 bps (5%)
pub const BPS_DENOMINATOR: u128 = 10_000;

#[program]
pub mod venpay {
    use super::*;

    /// Initializes the global protocol configuration PDA once.
    pub fn initialize_protocol(
        ctx: Context<InitializeProtocol>,
        treasury: Pubkey,
        fee_bps: u16,
    ) -> Result<()> {
        require!(
            treasury != Pubkey::default(),
            VenPayError::InvalidTreasuryAccount
        );
        require!(fee_bps <= MAX_FEE_BPS, VenPayError::FeeTooHigh);

        let config = &mut ctx.accounts.config;
        config.admin = ctx.accounts.admin.key();
        config.treasury = treasury;
        config.fee_bps = fee_bps;
        config.total_calls = 0;
        config.total_volume_lamports = 0;
        config.bump = ctx.bumps.config;

        emit!(ProtocolInitializedEvent {
            admin: config.admin,
            treasury: config.treasury,
            fee_bps,
        });

        Ok(())
    }

    /// Registers a curated, verified MCP tool on-chain with fixed lamport pricing.
    pub fn register_tool(
        ctx: Context<RegisterTool>,
        slug: String,
        provider: Pubkey,
        price_lamports: u64,
        schema_hash: [u8; 32],
    ) -> Result<()> {
        validate_slug(&slug)?;
        require!(
            provider != Pubkey::default(),
            VenPayError::InvalidProviderAccount
        );
        require!(price_lamports > 0, VenPayError::InvalidPrice);

        let tool = &mut ctx.accounts.tool;
        tool.slug = slug.clone();
        tool.provider = provider;
        tool.price_lamports = price_lamports;
        tool.schema_hash = schema_hash;
        tool.verified = true;
        tool.active = true;
        tool.total_calls = 0;
        tool.bump = ctx.bumps.tool;

        emit!(ToolRegisteredEvent {
            slug,
            provider,
            price_lamports,
            schema_hash,
        });

        Ok(())
    }

    /// Updates pricing, payout address, schema hash, or verification/active flags for an existing tool.
    pub fn update_tool(
        ctx: Context<UpdateTool>,
        provider: Pubkey,
        price_lamports: u64,
        schema_hash: [u8; 32],
        verified: bool,
        active: bool,
    ) -> Result<()> {
        require!(
            provider != Pubkey::default(),
            VenPayError::InvalidProviderAccount
        );
        require!(price_lamports > 0, VenPayError::InvalidPrice);

        let tool = &mut ctx.accounts.tool;
        tool.provider = provider;
        tool.price_lamports = price_lamports;
        tool.schema_hash = schema_hash;
        tool.verified = verified;
        tool.active = active;

        emit!(ToolUpdatedEvent {
            slug: tool.slug.clone(),
            provider,
            price_lamports,
            verified,
            active,
        });

        Ok(())
    }

    /// Creates a non-custodial AgentVault PDA owned by the caller with a scoped delegate and per-call cap.
    pub fn create_vault(
        ctx: Context<CreateVault>,
        delegate: Pubkey,
        max_per_call: u64,
    ) -> Result<()> {
        require!(delegate != Pubkey::default(), VenPayError::InvalidDelegate);
        require!(max_per_call > 0, VenPayError::InvalidAmount);

        let vault = &mut ctx.accounts.vault;
        vault.owner = ctx.accounts.owner.key();
        vault.delegate = delegate;
        vault.balance_lamports = 0;
        vault.max_per_call = max_per_call;
        vault.total_spent = 0;
        vault.call_count = 0;
        vault.active = true;
        vault.bump = ctx.bumps.vault;

        emit!(VaultCreatedEvent {
            vault: vault.key(),
            owner: vault.owner,
            delegate,
            max_per_call,
        });

        Ok(())
    }

    /// Rotates the scoped delegate key, updates the per-call spend cap, or pauses/resumes the vault.
    pub fn update_vault(
        ctx: Context<UpdateVault>,
        delegate: Pubkey,
        max_per_call: u64,
        active: bool,
    ) -> Result<()> {
        require!(delegate != Pubkey::default(), VenPayError::InvalidDelegate);
        require!(max_per_call > 0, VenPayError::InvalidAmount);

        let vault = &mut ctx.accounts.vault;
        vault.delegate = delegate;
        vault.max_per_call = max_per_call;
        vault.active = active;

        emit!(VaultUpdatedEvent {
            vault: vault.key(),
            owner: vault.owner,
            delegate,
            max_per_call,
            active,
        });

        Ok(())
    }

    /// Deposits SOL into an AgentVault PDA (callable by owner, demo faucet relayer, or top-up bridge).
    pub fn deposit_vault(ctx: Context<DepositVault>, amount: u64) -> Result<()> {
        require!(amount > 0, VenPayError::InvalidAmount);

        let cpi_ctx = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.depositor.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
            },
        );
        system_program::transfer(cpi_ctx, amount)?;

        let vault = &mut ctx.accounts.vault;
        vault.balance_lamports = vault
            .balance_lamports
            .checked_add(amount)
            .ok_or(VenPayError::MathOverflow)?;

        emit!(VaultDepositedEvent {
            vault: vault.key(),
            depositor: ctx.accounts.depositor.key(),
            amount_lamports: amount,
            new_balance_lamports: vault.balance_lamports,
        });

        Ok(())
    }

    /// Owner-only withdrawal from AgentVault while preserving rent-exempt minimum lamports on the PDA.
    pub fn withdraw_vault(ctx: Context<WithdrawVault>, amount: u64) -> Result<()> {
        require!(amount > 0, VenPayError::InvalidAmount);
        require!(
            ctx.accounts.vault.balance_lamports >= amount,
            VenPayError::InsufficientVaultBalance
        );

        let vault_info = ctx.accounts.vault.to_account_info();
        let owner_info = ctx.accounts.owner.to_account_info();
        let rent_min = Rent::get()?.minimum_balance(vault_info.data_len());
        let available_lamports = vault_info
            .lamports()
            .checked_sub(rent_min)
            .ok_or(VenPayError::InsufficientVaultBalance)?;

        require!(
            available_lamports >= amount,
            VenPayError::InsufficientVaultBalance
        );

        {
            let mut vault_lamports = vault_info.try_borrow_mut_lamports()?;
            **vault_lamports = vault_lamports
                .checked_sub(amount)
                .ok_or(VenPayError::InsufficientVaultBalance)?;
        }
        {
            let mut owner_lamports = owner_info.try_borrow_mut_lamports()?;
            **owner_lamports = owner_lamports
                .checked_add(amount)
                .ok_or(VenPayError::MathOverflow)?;
        }

        let vault = &mut ctx.accounts.vault;
        vault.balance_lamports = vault
            .balance_lamports
            .checked_sub(amount)
            .ok_or(VenPayError::InsufficientVaultBalance)?;

        emit!(VaultWithdrawnEvent {
            vault: vault.key(),
            owner: vault.owner,
            amount_lamports: amount,
            new_balance_lamports: vault.balance_lamports,
        });

        Ok(())
    }

    /// Atomically settles an x402 pay-per-call challenge from an AgentVault (95% provider / 5% treasury).
    pub fn settle_call(ctx: Context<SettleCall>, nonce: String) -> Result<()> {
        validate_nonce(&nonce)?;

        require!(ctx.accounts.vault.active, VenPayError::VaultInactive);
        require!(ctx.accounts.tool.active, VenPayError::ToolInactive);
        require!(ctx.accounts.tool.verified, VenPayError::ToolNotVerified);

        let price = ctx.accounts.tool.price_lamports;
        let fee_bps = ctx.accounts.config.fee_bps;

        require!(
            price <= ctx.accounts.vault.max_per_call,
            VenPayError::ExceedsMaxPerCall
        );
        require!(
            ctx.accounts.vault.balance_lamports >= price,
            VenPayError::InsufficientVaultBalance
        );

        let fee_u128 = (price as u128)
            .checked_mul(fee_bps as u128)
            .ok_or(VenPayError::MathOverflow)?
            .checked_div(BPS_DENOMINATOR)
            .ok_or(VenPayError::MathOverflow)?;
        let fee = u64::try_from(fee_u128).map_err(|_| VenPayError::MathOverflow)?;
        let provider_payout = price.checked_sub(fee).ok_or(VenPayError::MathOverflow)?;

        let vault_info = ctx.accounts.vault.to_account_info();
        let provider_info = ctx.accounts.provider.to_account_info();
        let treasury_info = ctx.accounts.treasury.to_account_info();

        let rent_min = Rent::get()?.minimum_balance(vault_info.data_len());
        let available_lamports = vault_info
            .lamports()
            .checked_sub(rent_min)
            .ok_or(VenPayError::InsufficientVaultBalance)?;
        require!(
            available_lamports >= price,
            VenPayError::InsufficientVaultBalance
        );

        // Deduct and credit in isolated borrow scopes so provider == treasury never triggers RefCell conflicts.
        {
            let mut vault_lamports = vault_info.try_borrow_mut_lamports()?;
            **vault_lamports = vault_lamports
                .checked_sub(price)
                .ok_or(VenPayError::InsufficientVaultBalance)?;
        }
        if provider_payout > 0 {
            let mut provider_lamports = provider_info.try_borrow_mut_lamports()?;
            **provider_lamports = provider_lamports
                .checked_add(provider_payout)
                .ok_or(VenPayError::MathOverflow)?;
        }
        if fee > 0 {
            let mut treasury_lamports = treasury_info.try_borrow_mut_lamports()?;
            **treasury_lamports = treasury_lamports
                .checked_add(fee)
                .ok_or(VenPayError::MathOverflow)?;
        }

        let vault = &mut ctx.accounts.vault;
        vault.balance_lamports = vault
            .balance_lamports
            .checked_sub(price)
            .ok_or(VenPayError::InsufficientVaultBalance)?;
        vault.total_spent = vault
            .total_spent
            .checked_add(price)
            .ok_or(VenPayError::MathOverflow)?;
        vault.call_count = vault
            .call_count
            .checked_add(1)
            .ok_or(VenPayError::MathOverflow)?;

        let tool = &mut ctx.accounts.tool;
        tool.total_calls = tool
            .total_calls
            .checked_add(1)
            .ok_or(VenPayError::MathOverflow)?;

        let config = &mut ctx.accounts.config;
        config.total_calls = config
            .total_calls
            .checked_add(1)
            .ok_or(VenPayError::MathOverflow)?;
        config.total_volume_lamports = config
            .total_volume_lamports
            .checked_add(price)
            .ok_or(VenPayError::MathOverflow)?;

        let timestamp = Clock::get()?.unix_timestamp;
        let vault_key = vault.key();
        let owner_key = vault.owner;
        let delegate_key = vault.delegate;
        let tool_slug = tool.slug.clone();

        msg!(
            "VENPAY_SETTLE|tool={}|vault={}|amount={}|fee={}|nonce={}",
            tool_slug,
            vault_key,
            price,
            fee,
            nonce
        );

        emit!(CallSettledEvent {
            vault: vault_key,
            owner: owner_key,
            delegate: delegate_key,
            signer: ctx.accounts.authority.key(),
            tool_slug,
            amount_lamports: price,
            fee_lamports: fee,
            provider_lamports: provider_payout,
            nonce,
            timestamp,
        });

        Ok(())
    }
}

fn validate_slug(slug: &str) -> Result<()> {
    require!(
        !slug.is_empty() && slug.len() <= MAX_SLUG_LEN,
        VenPayError::InvalidSlug
    );
    require!(
        slug.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-'),
        VenPayError::InvalidSlug
    );
    Ok(())
}

fn validate_nonce(nonce: &str) -> Result<()> {
    require!(
        !nonce.is_empty() && nonce.len() <= MAX_NONCE_LEN,
        VenPayError::InvalidNonce
    );
    require!(
        nonce
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-'),
        VenPayError::InvalidNonce
    );
    Ok(())
}

#[derive(Accounts)]
pub struct InitializeProtocol<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        space = ProtocolConfig::SPACE,
        seeds = [b"config"],
        bump
    )]
    pub config: Account<'info, ProtocolConfig>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(slug: String)]
pub struct RegisterTool<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        seeds = [b"config"],
        bump = config.bump,
        has_one = admin @ VenPayError::UnauthorizedAdmin,
    )]
    pub config: Account<'info, ProtocolConfig>,
    #[account(
        init,
        payer = admin,
        space = ToolRegistry::SPACE,
        seeds = [b"tool", slug.as_bytes()],
        bump
    )]
    pub tool: Account<'info, ToolRegistry>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateTool<'info> {
    pub admin: Signer<'info>,
    #[account(
        seeds = [b"config"],
        bump = config.bump,
        has_one = admin @ VenPayError::UnauthorizedAdmin,
    )]
    pub config: Account<'info, ProtocolConfig>,
    #[account(
        mut,
        seeds = [b"tool", tool.slug.as_bytes()],
        bump = tool.bump,
    )]
    pub tool: Account<'info, ToolRegistry>,
}

#[derive(Accounts)]
pub struct CreateVault<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        init,
        payer = owner,
        space = AgentVault::SPACE,
        seeds = [b"vault", owner.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, AgentVault>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateVault<'info> {
    pub owner: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault", owner.key().as_ref()],
        bump = vault.bump,
        has_one = owner @ VenPayError::UnauthorizedOwner,
    )]
    pub vault: Account<'info, AgentVault>,
}

#[derive(Accounts)]
pub struct DepositVault<'info> {
    #[account(mut)]
    pub depositor: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault", vault.owner.as_ref()],
        bump = vault.bump,
    )]
    pub vault: Account<'info, AgentVault>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WithdrawVault<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault", owner.key().as_ref()],
        bump = vault.bump,
        has_one = owner @ VenPayError::UnauthorizedOwner,
    )]
    pub vault: Account<'info, AgentVault>,
}

#[derive(Accounts)]
pub struct SettleCall<'info> {
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"config"],
        bump = config.bump,
    )]
    pub config: Account<'info, ProtocolConfig>,
    #[account(
        mut,
        seeds = [b"tool", tool.slug.as_bytes()],
        bump = tool.bump,
    )]
    pub tool: Account<'info, ToolRegistry>,
    #[account(
        mut,
        seeds = [b"vault", vault.owner.as_ref()],
        bump = vault.bump,
        constraint = (authority.key() == vault.owner || authority.key() == vault.delegate) @ VenPayError::UnauthorizedSigner,
    )]
    pub vault: Account<'info, AgentVault>,
    /// CHECK: Verified via constraint against tool.provider payout address and distinct from vault PDA.
    #[account(
        mut,
        constraint = provider.key() == tool.provider @ VenPayError::InvalidProviderAccount,
        constraint = provider.key() != vault.key() @ VenPayError::InvalidProviderAccount,
    )]
    pub provider: UncheckedAccount<'info>,
    /// CHECK: Verified via constraint against config.treasury payout address and distinct from vault PDA.
    #[account(
        mut,
        constraint = treasury.key() == config.treasury @ VenPayError::InvalidTreasuryAccount,
        constraint = treasury.key() != vault.key() @ VenPayError::InvalidTreasuryAccount,
    )]
    pub treasury: UncheckedAccount<'info>,
}

#[account]
pub struct ProtocolConfig {
    pub admin: Pubkey,
    pub treasury: Pubkey,
    pub fee_bps: u16,
    pub total_calls: u64,
    pub total_volume_lamports: u64,
    pub bump: u8,
}

impl ProtocolConfig {
    pub const SPACE: usize = 8 + 32 + 32 + 2 + 8 + 8 + 1;
}

#[account]
pub struct ToolRegistry {
    pub slug: String,
    pub provider: Pubkey,
    pub price_lamports: u64,
    pub schema_hash: [u8; 32],
    pub verified: bool,
    pub active: bool,
    pub total_calls: u64,
    pub bump: u8,
}

impl ToolRegistry {
    pub const SPACE: usize = 8 + (4 + MAX_SLUG_LEN) + 32 + 8 + 32 + 1 + 1 + 8 + 1;
}

#[account]
pub struct AgentVault {
    pub owner: Pubkey,
    pub delegate: Pubkey,
    pub balance_lamports: u64,
    pub max_per_call: u64,
    pub total_spent: u64,
    pub call_count: u64,
    pub active: bool,
    pub bump: u8,
}

impl AgentVault {
    pub const SPACE: usize = 8 + 32 + 32 + 8 + 8 + 8 + 8 + 1 + 1;
}

#[event]
pub struct ProtocolInitializedEvent {
    pub admin: Pubkey,
    pub treasury: Pubkey,
    pub fee_bps: u16,
}

#[event]
pub struct ToolRegisteredEvent {
    pub slug: String,
    pub provider: Pubkey,
    pub price_lamports: u64,
    pub schema_hash: [u8; 32],
}

#[event]
pub struct ToolUpdatedEvent {
    pub slug: String,
    pub provider: Pubkey,
    pub price_lamports: u64,
    pub verified: bool,
    pub active: bool,
}

#[event]
pub struct VaultCreatedEvent {
    pub vault: Pubkey,
    pub owner: Pubkey,
    pub delegate: Pubkey,
    pub max_per_call: u64,
}

#[event]
pub struct VaultUpdatedEvent {
    pub vault: Pubkey,
    pub owner: Pubkey,
    pub delegate: Pubkey,
    pub max_per_call: u64,
    pub active: bool,
}

#[event]
pub struct VaultDepositedEvent {
    pub vault: Pubkey,
    pub depositor: Pubkey,
    pub amount_lamports: u64,
    pub new_balance_lamports: u64,
}

#[event]
pub struct VaultWithdrawnEvent {
    pub vault: Pubkey,
    pub owner: Pubkey,
    pub amount_lamports: u64,
    pub new_balance_lamports: u64,
}

#[event]
pub struct CallSettledEvent {
    pub vault: Pubkey,
    pub owner: Pubkey,
    pub delegate: Pubkey,
    pub signer: Pubkey,
    pub tool_slug: String,
    pub amount_lamports: u64,
    pub fee_lamports: u64,
    pub provider_lamports: u64,
    pub nonce: String,
    pub timestamp: i64,
}

#[error_code]
pub enum VenPayError {
    #[msg("Protocol fee basis points exceed the 20% safety cap.")]
    FeeTooHigh,
    #[msg("Only the protocol admin can perform this action.")]
    UnauthorizedAdmin,
    #[msg("Only the vault owner can perform this action.")]
    UnauthorizedOwner,
    #[msg("Signer is neither the vault owner nor the active delegate.")]
    UnauthorizedSigner,
    #[msg("Tool slug must be 1..=32 ASCII lowercase, digits, '_' or '-'.")]
    InvalidSlug,
    #[msg("Nonce must be 1..=64 ASCII alphanumeric, '_' or '-'.")]
    InvalidNonce,
    #[msg("Tool price must be greater than zero.")]
    InvalidPrice,
    #[msg("Amount or cap must be greater than zero.")]
    InvalidAmount,
    #[msg("Delegate public key cannot be the default address.")]
    InvalidDelegate,
    #[msg("Provider payout account does not match registered tool provider.")]
    InvalidProviderAccount,
    #[msg("Treasury payout account does not match protocol config treasury.")]
    InvalidTreasuryAccount,
    #[msg("AgentVault is currently paused.")]
    VaultInactive,
    #[msg("Tool is currently inactive.")]
    ToolInactive,
    #[msg("Tool is not marked as verified.")]
    ToolNotVerified,
    #[msg("Tool price exceeds the AgentVault max_per_call spending cap.")]
    ExceedsMaxPerCall,
    #[msg("AgentVault balance is insufficient for this operation.")]
    InsufficientVaultBalance,
    #[msg("Arithmetic overflow occurred.")]
    MathOverflow,
}
