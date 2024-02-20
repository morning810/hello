use anchor_lang::prelude::*;

declare_id!("BD79ZdPY9PmkbqJspSx7rDC8VPZmcQ9V6zGgsKFJFTJh");

#[program]
pub mod hello {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
