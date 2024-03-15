use anchor_lang::prelude::*;
use secp256k1::{recover, Message, RecoveryId, Signature};
declare_id!("BD79ZdPY9PmkbqJspSx7rDC8VPZmcQ9V6zGgsKFJFTJh");

use anchor_spl::token::{self, Mint, TokenAccount, Transfer};
use solana_program::program_pack::Pack;

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub bridge_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub bridge_state: Account<'info, BridgeState>,
    #[account(address = token::ID)]
    pub token_program: Program<'info, token::Token>,
}

#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(mut)]
    pub bridge_state: Account<'info, BridgeState>,
    #[account(mut)]
    pub bridge_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub recipient_token_account: Account<'info, TokenAccount>,
    #[account(address = token::ID)]
    pub token_program: Program<'info, token::Token>,
    /// CHECK: This is only used for its address.
    pub oracle: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = user, space = 8 + 32 + 32 + 8)] // Adjust space as needed
    pub bridge_state: Account<'info, BridgeState>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct BridgeState {
    pub oracle_address: Pubkey,
    pub fee_percent: u64,
    pub paused: bool, // Added pausing state
}

impl<'info> Initialize<'info> {
    pub fn process(&mut self, oracle_address: Pubkey, fee_percent: u64) -> Result<()> {
        self.bridge_state.oracle_address = oracle_address;
        self.bridge_state.fee_percent = fee_percent;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct UpdateOracleAddress<'info> {
    #[account(mut, has_one = owner)]
    pub bridge_state: Account<'info, BridgeState>,
    pub owner: Signer<'info>,
}

impl<'info> UpdateOracleAddress<'info> {
    pub fn process(&mut self, new_oracle_address: Pubkey) -> Result<()> {
        self.bridge_state.oracle_address = new_oracle_address;
        Ok(())
    }
}

// Function to pause the program
#[derive(Accounts)]
pub struct Pause<'info> {
    #[account(mut, has_one = owner)]
    pub bridge_state: Account<'info, BridgeState>,
    pub owner: Signer<'info>,
}

impl<'info> Pause<'info> {
    pub fn process(&mut self) -> Result<()> {
        self.bridge_state.paused = true;
        Ok(())
    }
}

// Function to unpause the program
#[derive(Accounts)]
pub struct Unpause<'info> {
    #[account(mut, has_one = owner)]
    pub bridge_state: Account<'info, BridgeState>,
    pub owner: Signer<'info>,
}

impl<'info> Unpause<'info> {
    pub fn process(&mut self) -> Result<()> {
        self.bridge_state.paused = false;
        Ok(())
    }
}

pub enum ErrorCode {
    #[msg("The contract is currently paused.")]
    ContractPaused,
    #[msg("Unauthorized access attempt.")]
    Unauthorized,
    #[msg("Invalid operation amount.")]
    InvalidAmount,
    #[msg("State consistency check failed.")]
    BalanceMismatch,
    #[msg("The operation is currently not allowed.")]
    OperationNotAllowed,
}

#[event]
pub struct TokenSwap {
    pub from: Pubkey,
    pub to: Pubkey,
    pub amount: u64,
}

#[program]
pub mod tear_bridge {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        oracle_address: Pubkey,
        fee_percent: u64,
    ) -> Result<()> {
        let bridge_state = &mut ctx.accounts.bridge_state;
        bridge_state.oracle_address = oracle_address;
        bridge_state.fee_percent = fee_percent;
        Ok(())
    }

    // Add other functions like swap, claim, set_oracle_address, etc.
    pub fn swap(ctx: Context<Swap>, &mut self, amount: u64, nonce: u64) -> Result<()> {        
        if !ctx.accounts.user.to_account_info().is_signer {
            return Err(ErrorCode::Unauthorized.into());
        }

        let bridge_state = &mut ctx.accounts.bridge_state;

        // Optional: Check if bridge is paused and handle nonce verification
        if self.bridge_state.paused {
            return Err(ErrorCode::ContractPaused.into());
        }
        // Transfer tokens from the user's account to the bridge's account
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.bridge_token_account.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        // Optional: Record the swap operation or emit an event
        emit!(TokenSwap {
            from: *ctx.accounts.user.to_account_info().key,
            to: *ctx.accounts.recipient.to_account_info().key,
            amount,
        });

        Ok(())
    }

    pub fn claim(ctx: Context<Claim>, amount: u64, nonce: u64, fee: u64) -> Result<()> {
        let bridge_state = &mut ctx.accounts.bridge_state;

        // Check if the oracle is the signer of this transaction
        require!(
            ctx.accounts.oracle.is_signer,
            "Oracle must sign the transaction"
        );

        // Verify nonce and update it to prevent replay attacks
        // This logic needs to be adapted based on your nonce management strategy

        // Calculate final amount after deducting fee
        let final_amount = amount.checked_sub(fee).ok_or(ErrorCode::InvalidAmount)?;

        // Transfer tokens from the bridge's account to the recipient's account
        let cpi_accounts = Transfer {
            from: ctx.accounts.bridge_token_account.to_account_info(),
            to: ctx.accounts.recipient_token_account.to_account_info(),
            authority: ctx.accounts.bridge_state.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, final_amount)?;

        // Optionally: Emit an event or record the claim for tracking

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = user, space = 8 + 32 + 8)]
    pub bridge_state: Account<'info, BridgeState>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct BridgeState {
    pub oracle_address: Pubkey,
    pub fee_percent: u64,
    // Additional fields like token_mint, claim_nonces, etc.
}

// Implement swap, claim, and utility functions here.
