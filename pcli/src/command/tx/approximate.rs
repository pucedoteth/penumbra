use std::path::PathBuf;

use crate::dex_utils;
use crate::dex_utils::approximate::debug;
use anyhow::anyhow;
use penumbra_crypto::dex::lp::position::Position;
use penumbra_crypto::dex::DirectedUnitPair;
use penumbra_crypto::Value;
use std::io::Write;

/// Queries the chain for a transaction by hash.
#[derive(Debug, clap::Subcommand)]
pub enum ApproximateCmd {
    #[clap(visible_alias = "xyk")]
    ConstantProduct(ConstantProduct),
}

#[derive(Debug, Clone, clap::Args)]
pub struct ConstantProduct {
    pub pair: DirectedUnitPair,
    pub input: Value,

    #[clap(short, long)]
    pub current_price: Option<f64>,

    #[clap(short, long, default_value_t = 0u32)]
    pub fee_bps: u32,
    /// `--yes` means all prompt interaction are skipped and agreed.
    #[clap(short, long)]
    pub yes: bool,

    #[clap(short, long, hide(true))]
    pub debug_file: Option<PathBuf>,
    #[clap(long, default_value = "0", hide(true))]
    pub source: u32,
}

impl ApproximateCmd {
    pub fn _offline(&self) -> bool {
        false
    }
}

impl ConstantProduct {
    pub fn exec(&self, current_price: f64) -> anyhow::Result<Vec<Position>> {
        if self.input.asset_id != self.pair.start.id() && self.input.asset_id != self.pair.end.id()
        {
            anyhow::bail!("you must supply liquidity with an asset that's part of the market")
        } else if self.input.amount == 0u64.into() {
            anyhow::bail!("the quantity of liquidity supplied must be non-zero.",)
        } else if self.fee_bps > 5000 {
            anyhow::bail!("the maximum fee is 5000bps (50%)")
        } else {
            let positions = crate::dex_utils::approximate::xyk::approximate(
                &self.pair,
                &self.input,
                current_price.try_into().expect("valid price"),
                self.fee_bps,
            )?;

            if let Some(file) = &self.debug_file {
                // Ad-hoc denom scaling for debug data:
                let alphas = dex_utils::approximate::xyk::sample_full_range(
                    current_price,
                    dex_utils::approximate::xyk::NUM_POOLS_PRECISION,
                );

                alphas
                    .iter()
                    .enumerate()
                    .for_each(|(i, alpha)| tracing::debug!(i, alpha, "sampled tick"));

                // TODO(erwan): this is wrong, i should rip this out anyway and not have two places that do the denom scaling logic.
                let r1 = self.input.amount.value() as f64;
                // R2 scaled because the current_price is a ratio.
                let r2 = r1 * current_price;
                let total_k = r1 * r2;
                tracing::debug!(r1, r2, total_k, current_price);

                let debug_positions: Vec<debug::PayoffPositionEntry> = positions
                    .iter()
                    .zip(alphas)
                    .enumerate()
                    .map(|(idx, (pos, alpha))| debug::PayoffPositionEntry {
                        payoff: Into::into(pos.clone()),
                        current_price,
                        index: idx,
                        pair: self.pair.clone(),
                        alpha,
                        total_k,
                    })
                    .collect();

                let mut fd = std::fs::File::create(&file).map_err(|e| {
                    anyhow!(
                        "fs error opening debug file {}: {}",
                        file.to_string_lossy(),
                        e
                    )
                })?;

                let json_data = serde_json::to_string(&debug_positions)
                    .map_err(|e| anyhow!("error serializing PayoffPositionEntry: {}", e))?;

                fd.write_all(json_data.as_bytes())
                    .map_err(|e| anyhow!("error writing {}: {}", file.to_string_lossy(), e))?;
            }
            Ok(positions)
        }
    }
}