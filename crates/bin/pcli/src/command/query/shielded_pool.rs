use anyhow::Result;
use colored_json::prelude::*;
use penumbra_compact_block::CompactBlock;
use penumbra_proto::DomainType;
use penumbra_sct::{CommitmentSource, NullificationInfo, Nullifier};
use penumbra_tct::StateCommitment;

#[derive(Debug, clap::Subcommand)]
pub enum ShieldedPool {
    /// Queries the state commitment tree anchor for a given height.
    Anchor {
        /// The height to query.
        height: u64,
    },
    /// Queries the source of a given commitment.
    Commitment {
        /// The commitment to query.
        #[clap(parse(try_from_str = StateCommitment::parse_hex))]
        commitment: StateCommitment,
    },
    /// Queries the note source of a given nullifier.
    Nullifier {
        /// The nullifier to query.
        #[clap(parse(try_from_str = Nullifier::parse_hex))]
        nullifier: Nullifier,
    },
    /// Queries the compact block at a given height.
    CompactBlock { height: u64 },
}

impl ShieldedPool {
    pub fn key(&self) -> String {
        use penumbra_compact_block::state_key as cb_state_key;
        use penumbra_sct::state_key as sct_state_key;
        match self {
            ShieldedPool::Anchor { height } => sct_state_key::anchor_by_height(*height),
            ShieldedPool::CompactBlock { height } => cb_state_key::compact_block(*height),
            ShieldedPool::Commitment { commitment } => sct_state_key::note_source(commitment),
            ShieldedPool::Nullifier { nullifier } => {
                sct_state_key::spent_nullifier_lookup(nullifier)
            }
        }
    }

    pub fn display_value(&self, bytes: &[u8]) -> Result<()> {
        let json = match self {
            ShieldedPool::Anchor { .. } => {
                let anchor = penumbra_tct::Root::decode(bytes)?;
                serde_json::to_string_pretty(&anchor)?
            }
            ShieldedPool::CompactBlock { .. } => {
                let compact_block = CompactBlock::decode(bytes)?;
                serde_json::to_string_pretty(&compact_block)?
            }
            ShieldedPool::Commitment { .. } => {
                let commitment_source = CommitmentSource::decode(bytes)?;
                serde_json::to_string_pretty(&commitment_source)?
            }
            ShieldedPool::Nullifier { .. } => {
                let note_source = NullificationInfo::decode(bytes)?;
                serde_json::to_string_pretty(&note_source)?
            }
        };
        println!("{}", json.to_colored_json_auto()?);
        Ok(())
    }
}
