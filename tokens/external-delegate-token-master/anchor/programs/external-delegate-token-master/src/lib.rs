use anchor_lang::prelude::*;
use sha3::{Digest, Keccak256};
use solana_secp256k1_recover::secp256k1_recover;

mod instructions;
use instructions::*;

declare_id!("FYPkt5VWMvtyWZDMGCwoKFkE3wXTzphicTpnNGuHWVbD");

#[program]
pub mod external_delegate_token_master {
    use super::*;

    pub fn initialize(context: Context<Initialize>) -> Result<()> {
        instructions::initialize::handler(context)
    }

    pub fn set_ethereum_address(
        context: Context<SetEthereumAddress>,
        ethereum_address: [u8; 20],
    ) -> Result<()> {
        instructions::set_ethereum_address::handler(context, ethereum_address)
    }

    pub fn transfer_tokens(
        context: Context<TransferTokens>,
        amount: u64,
        signature: [u8; 65],
        message: [u8; 32],
    ) -> Result<()> {
        instructions::transfer_tokens::handler(context, amount, signature, message)
    }

    pub fn authority_transfer(context: Context<AuthorityTransfer>, amount: u64) -> Result<()> {
        instructions::authority_transfer::handler(context, amount)
    }
}

#[account]
#[derive(InitSpace)]
pub struct UserAccount {
    pub authority: Pubkey,
    pub ethereum_address: [u8; 20],
}

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid Ethereum signature")]
    InvalidSignature,
}

pub fn verify_ethereum_signature(
    ethereum_address: &[u8; 20],
    message: &[u8; 32],
    signature: &[u8; 65],
) -> bool {
    let recovery_id = signature[64];
    let mut sig = [0u8; 64];
    sig.copy_from_slice(&signature[..64]);

    if let Ok(pubkey) = secp256k1_recover(message, recovery_id, &sig) {
        let pubkey_bytes = pubkey.to_bytes();
        let mut recovered_address = [0u8; 20];
        recovered_address.copy_from_slice(&keccak256(&pubkey_bytes[1..])[12..]);
        recovered_address == *ethereum_address
    } else {
        false
    }
}

fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(data);
    hasher.finalize().into()
}
