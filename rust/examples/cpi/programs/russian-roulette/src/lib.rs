mod misc;
mod state;

use anchor_lang::prelude::*;
use orao_solana_vrf::program::OraoVrf;
use orao_solana_vrf::state::NetworkState;
use orao_solana_vrf::CONFIG_ACCOUNT_SEED;
use orao_solana_vrf::RANDOMNESS_ACCOUNT_SEED;
use state::PlayerState;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

pub const PLAYER_STATE_ACCOUNT_SEED: &[u8] = b"russian-roulette-player-state";

#[program]
pub mod russian_roulette {
    use orao_solana_vrf::cpi::accounts::Request;

    use super::*;

    pub fn spin_and_pull_the_trigger(
        ctx: Context<SpinAndThePullTheTrigger>,
        force: [u8; 32],
    ) -> Result<()> {
        // Zero seed is illegal in VRF
        if force == [0_u8; 32] {
            return Err(Error::YouMustSpinTheCylinder.into());
        }

        let player_state = &mut ctx.accounts.player_state;

        // initialize
        if player_state.rounds == 0 {
            player_state.player = *ctx.accounts.player.as_ref().key;
        }

        // Assert that the player is able to play.
        player_state.assert_can_play(ctx.accounts.prev_round.as_ref())?;

        // Request randomness.
        let cpi_program = ctx.accounts.vrf.to_account_info();
        let cpi_accounts = Request {
            payer: ctx.accounts.player.to_account_info(),
            network_state: ctx.accounts.config.to_account_info(),
            treasury: ctx.accounts.treasury.to_account_info(),
            request: ctx.accounts.random.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        orao_solana_vrf::cpi::request(cpi_ctx, force)?;

        player_state.rounds += 1;
        player_state.force = force;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(force: [u8; 32])]
pub struct SpinAndThePullTheTrigger<'info> {
    #[account(mut)]
    player: Signer<'info>,
    #[account(
        init_if_needed,
        payer = player,
        space = 8 + PlayerState::SIZE,
        seeds = [
            PLAYER_STATE_ACCOUNT_SEED,
            player.key().as_ref()
        ],
        bump
    )]
    player_state: Account<'info, PlayerState>,
    /// CHECK:
    #[account(
        seeds = [RANDOMNESS_ACCOUNT_SEED.as_ref(), player_state.force.as_ref()],
        bump,
        seeds::program = orao_solana_vrf::ID
    )]
    prev_round: AccountInfo<'info>,
    /// CHECK:
    #[account(
        mut,
        seeds = [RANDOMNESS_ACCOUNT_SEED.as_ref(), &force],
        bump,
        seeds::program = orao_solana_vrf::ID
    )]
    random: AccountInfo<'info>,
    /// CHECK:
    #[account(mut)]
    treasury: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [CONFIG_ACCOUNT_SEED.as_ref()],
        bump,
        seeds::program = orao_solana_vrf::ID
    )]
    config: Account<'info, NetworkState>,
    vrf: Program<'info, OraoVrf>,
    system_program: Program<'info, System>,
}

#[error_code]
pub enum Error {
    #[msg("The player is already dead")]
    PlayerDead,
    #[msg("Unable to serialize a randomness request")]
    RandomnessRequestSerializationError,
    #[msg("Player must spin the cylinder")]
    YouMustSpinTheCylinder,
    #[msg("The cylinder is still spinning")]
    TheCylinderIsStillSpinning,
}