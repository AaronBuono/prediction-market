use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("9KQjnCXwNcnaojsfvuD894UjnCKvgwEDe4Kt1nfpDNHB");

#[program]
pub mod prediction_market {
    use super::*;

    pub fn create_market(
        ctx: Context<CreateMarket>,
        market_id: u64,
        description: String,
        end_time: i64,
        min_bet_amount: u64,
    ) -> Result<()> {
        let market = &mut ctx.accounts.market;
        let clock = Clock::get()?;

        // Validate inputs
        require!(end_time > clock.unix_timestamp, ErrorCode::InvalidEndTime);
        require!(description.len() <= 280, ErrorCode::DescriptionTooLong);
        require!(min_bet_amount > 0, ErrorCode::InvalidBetAmount);

        // Initialize market
        market.authority = ctx.accounts.authority.key();
        market.market_id = market_id;
        market.description = description;
        market.end_time = end_time;
        market.min_bet_amount = min_bet_amount;
        market.total_yes_bets = 0;
        market.total_no_bets = 0;
        market.is_resolved = false;
        market.winning_outcome = None;
        market.created_at = clock.unix_timestamp;

        emit!(MarketCreated {
            market_id,
            authority: ctx.accounts.authority.key(),
            description: market.description.clone(),
            end_time,
        });

        Ok(())
    }

    pub fn place_bet(
        ctx: Context<PlaceBet>,
        market_id: u64,
        bet_outcome: bool, // true for YES, false for NO
        amount: u64,
    ) -> Result<()> {
        let market = &mut ctx.accounts.market;
        let bet = &mut ctx.accounts.bet;
        let clock = Clock::get()?;

        // Validate market state
        require!(!market.is_resolved, ErrorCode::MarketAlreadyResolved);
        require!(clock.unix_timestamp < market.end_time, ErrorCode::MarketExpired);
        require!(amount >= market.min_bet_amount, ErrorCode::BetTooSmall);
        require!(market.market_id == market_id, ErrorCode::InvalidMarketId);

        // Transfer tokens from bettor to market vault
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.bettor_token_account.to_account_info(),
                to: ctx.accounts.market_vault.to_account_info(),
                authority: ctx.accounts.bettor.to_account_info(),
            },
        );
        token::transfer(transfer_ctx, amount)?;

        // Update market totals
        if bet_outcome {
            market.total_yes_bets = market.total_yes_bets.checked_add(amount).unwrap();
        } else {
            market.total_no_bets = market.total_no_bets.checked_add(amount).unwrap();
        }

        // Initialize bet account
        bet.bettor = ctx.accounts.bettor.key();
        bet.market_id = market_id;
        bet.outcome = bet_outcome;
        bet.amount = amount;
        bet.timestamp = clock.unix_timestamp;
        bet.is_claimed = false;

        emit!(BetPlaced {
            market_id,
            bettor: ctx.accounts.bettor.key(),
            outcome: bet_outcome,
            amount,
        });

        Ok(())
    }

    pub fn resolve_market(
        ctx: Context<ResolveMarket>,
        market_id: u64,
        winning_outcome: bool,
    ) -> Result<()> {
        let market = &mut ctx.accounts.market;
        let clock = Clock::get()?;

        // Validate authority
        require!(
            ctx.accounts.authority.key() == market.authority,
            ErrorCode::UnauthorizedResolver
        );

        // Validate market state
        require!(!market.is_resolved, ErrorCode::MarketAlreadyResolved);
        require!(clock.unix_timestamp >= market.end_time, ErrorCode::MarketNotExpired);
        require!(market.market_id == market_id, ErrorCode::InvalidMarketId);

        // Resolve market
        market.is_resolved = true;
        market.winning_outcome = Some(winning_outcome);

        emit!(MarketResolved {
            market_id,
            winning_outcome,
            resolver: ctx.accounts.authority.key(),
        });

        Ok(())
    }

    pub fn claim_winnings(
        ctx: Context<ClaimWinnings>,
        market_id: u64,
    ) -> Result<()> {
        let market = &ctx.accounts.market;
        let bet = &mut ctx.accounts.bet;

        // Validate market and bet state
        require!(market.is_resolved, ErrorCode::MarketNotResolved);
        require!(!bet.is_claimed, ErrorCode::AlreadyClaimed);
        require!(bet.market_id == market_id, ErrorCode::InvalidMarketId);
        require!(
            bet.bettor == ctx.accounts.bettor.key(),
            ErrorCode::UnauthorizedClaimer
        );

        // Check if bet won
        let winning_outcome = market.winning_outcome.unwrap();
        require!(bet.outcome == winning_outcome, ErrorCode::LosingBet);

        // Calculate winnings
        let total_pool = market.total_yes_bets + market.total_no_bets;
        let winning_pool = if winning_outcome {
            market.total_yes_bets
        } else {
            market.total_no_bets
        };

        let winnings = (bet.amount as u128)
            .checked_mul(total_pool as u128).unwrap()
            .checked_div(winning_pool as u128).unwrap() as u64;

        // Transfer winnings
        let market_key = market.key();
        let seeds = &[
            b"market_vault",
            market_key.as_ref(),
            &[ctx.bumps.market_vault],
        ];
        let signer_seeds = &[&seeds[..]];

        let transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.market_vault.to_account_info(),
                to: ctx.accounts.bettor_token_account.to_account_info(),
                authority: ctx.accounts.market_vault.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(transfer_ctx, winnings)?;

        // Mark as claimed
        bet.is_claimed = true;

        emit!(WinningsClaimed {
            market_id,
            bettor: ctx.accounts.bettor.key(),
            amount: winnings,
        });

        Ok(())
    }
}

// Account structures
#[derive(Accounts)]
#[instruction(market_id: u64)]
pub struct CreateMarket<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + Market::INIT_SPACE,
        seeds = [b"market", market_id.to_le_bytes().as_ref()],
        bump
    )]
    pub market: Account<'info, Market>,
    
    #[account(
        init,
        payer = authority,
        seeds = [b"market_vault", market.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = market_vault,
    )]
    pub market_vault: Account<'info, TokenAccount>,
    
    pub mint: Account<'info, anchor_spl::token::Mint>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(market_id: u64)]
pub struct PlaceBet<'info> {
    #[account(
        mut,
        seeds = [b"market", market_id.to_le_bytes().as_ref()],
        bump
    )]
    pub market: Account<'info, Market>,
    
    #[account(
        init,
        payer = bettor,
        space = 8 + Bet::INIT_SPACE,
        seeds = [b"bet", market.key().as_ref(), bettor.key().as_ref()],
        bump
    )]
    pub bet: Account<'info, Bet>,
    
    #[account(
        mut,
        seeds = [b"market_vault", market.key().as_ref()],
        bump
    )]
    pub market_vault: Account<'info, TokenAccount>,
    
    #[account(mut)]
    pub bettor_token_account: Account<'info, TokenAccount>,
    
    #[account(mut)]
    pub bettor: Signer<'info>,
    
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(market_id: u64)]
pub struct ResolveMarket<'info> {
    #[account(
        mut,
        seeds = [b"market", market_id.to_le_bytes().as_ref()],
        bump
    )]
    pub market: Account<'info, Market>,
    
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(market_id: u64)]
pub struct ClaimWinnings<'info> {
    #[account(
        seeds = [b"market", market_id.to_le_bytes().as_ref()],
        bump
    )]
    pub market: Account<'info, Market>,
    
    #[account(
        mut,
        seeds = [b"bet", market.key().as_ref(), bettor.key().as_ref()],
        bump
    )]
    pub bet: Account<'info, Bet>,
    
    #[account(
        mut,
        seeds = [b"market_vault", market.key().as_ref()],
        bump
    )]
    pub market_vault: Account<'info, TokenAccount>,
    
    #[account(mut)]
    pub bettor_token_account: Account<'info, TokenAccount>,
    
    pub bettor: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

// Data structures
#[account]
#[derive(InitSpace)]
pub struct Market {
    pub authority: Pubkey,
    pub market_id: u64,
    #[max_len(280)]
    pub description: String,
    pub end_time: i64,
    pub min_bet_amount: u64,
    pub total_yes_bets: u64,
    pub total_no_bets: u64,
    pub is_resolved: bool,
    pub winning_outcome: Option<bool>,
    pub created_at: i64,
}

#[account]
#[derive(InitSpace)]
pub struct Bet {
    pub bettor: Pubkey,
    pub market_id: u64,
    pub outcome: bool,
    pub amount: u64,
    pub timestamp: i64,
    pub is_claimed: bool,
}

// Events
#[event]
pub struct MarketCreated {
    pub market_id: u64,
    pub authority: Pubkey,
    pub description: String,
    pub end_time: i64,
}

#[event]
pub struct BetPlaced {
    pub market_id: u64,
    pub bettor: Pubkey,
    pub outcome: bool,
    pub amount: u64,
}

#[event]
pub struct MarketResolved {
    pub market_id: u64,
    pub winning_outcome: bool,
    pub resolver: Pubkey,
}

#[event]
pub struct WinningsClaimed {
    pub market_id: u64,
    pub bettor: Pubkey,
    pub amount: u64,
}

// Error codes
#[error_code]
pub enum ErrorCode {
    #[msg("Invalid end time - must be in the future")]
    InvalidEndTime,
    #[msg("Description too long - max 280 characters")]
    DescriptionTooLong,
    #[msg("Invalid bet amount")]
    InvalidBetAmount,
    #[msg("Market already resolved")]
    MarketAlreadyResolved,
    #[msg("Market has expired")]
    MarketExpired,
    #[msg("Bet amount too small")]
    BetTooSmall,
    #[msg("Invalid market ID")]
    InvalidMarketId,
    #[msg("Unauthorized resolver")]
    UnauthorizedResolver,
    #[msg("Market not expired yet")]
    MarketNotExpired,
    #[msg("Market not resolved yet")]
    MarketNotResolved,
    #[msg("Winnings already claimed")]
    AlreadyClaimed,
    #[msg("Unauthorized claimer")]
    UnauthorizedClaimer,
    #[msg("This bet lost")]
    LosingBet,
}
