# Solana Prediction Market Smart Contract

A decentralized prediction market built with Anchor framework for Solana Hackaroo.

## Features

- ✅ **Create Market**: Anyone can create a prediction market with a description and end time
- ✅ **Place Bet**: Users can bet YES or NO on market outcomes using SPL tokens
- ✅ **Resolve Market**: Market creators can resolve markets after the end time
- ✅ **Claim Winnings**: Winners can claim their proportional share of the total pool

## Smart Contract Functions

1. **`createMarket()`** - Creates a new prediction market
2. **`placeBet()`** - Places a bet on YES or NO outcome  
3. **`resolveMarket()`** - Resolves the market with winning outcome
4. **`claimWinnings()`** - Allows winners to claim their rewards

## Quick Start

### Prerequisites
- Rust: https://rustup.rs/
- Solana CLI: https://docs.solana.com/cli/install-solana-cli-tools
- Anchor: https://www.anchor-lang.com/docs/installation
- Node.js: https://nodejs.org/

### Setup & Deploy

```bash
# Install dependencies
npm install

# Configure Solana for devnet
solana config set --url devnet
solana-keygen new  # if you don't have a keypair
solana airdrop 2   # get some devnet SOL

# Build and deploy
anchor build
anchor deploy
```

### Push to GitHub

```bash
git add .
git commit -m "Solana prediction market smart contract"
git remote add origin https://github.com/yourusername/prediction-market.git
git push -u origin main
```

## Contract Architecture

- **Security**: Time-based validation, authority checks, prevents double-claiming
- **Economics**: Proportional winnings distribution from total pool
- **Integration**: SPL token support, event emissions for frontends

Built for Solana Hackaroo 🚀
# prediction-market
