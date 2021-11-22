use crate::contracts::fee_manager_contract::*;

use super::types::FeeManagerState;

use offchain_core::types::Block;
use state_fold::{
    delegate_access::{FoldAccess, SyncAccess},
    error::*,
    types::*,
    utils as fold_utils,
};

use async_trait::async_trait;
use snafu::ResultExt;

use ethers::prelude::EthEvent;
use ethers::types::{Address, U256};
use ethers_core::types::I256;

use im::HashMap;

/// Fee Manager Delegate
#[derive(Default)]
pub struct FeeManagerDelegate {}

/// voucher_position = voucher_index * 2 ** 128 + input_index * 2 ** 64 + epoch
/// We always assume indices have at most 8 bytes, as does rust
fn convert_voucher_position_to_indices(
    voucher_position: U256,
) -> (usize, usize, usize) {
    let mut pos_bytes = [0u8; 32];
    voucher_position.to_big_endian(&mut pos_bytes);

    let mut voucher_index_bytes = [0u8; 8];
    voucher_index_bytes.copy_from_slice(&pos_bytes[8..16]);

    let mut input_index_bytes = [0u8; 8];
    input_index_bytes.copy_from_slice(&pos_bytes[16..24]);

    let mut epoch_bytes = [0u8; 8];
    epoch_bytes.copy_from_slice(&pos_bytes[24..32]);

    (
        usize::from_be_bytes(voucher_index_bytes),
        usize::from_be_bytes(input_index_bytes),
        usize::from_be_bytes(epoch_bytes),
    )
}

#[async_trait]
impl StateFoldDelegate for FeeManagerDelegate {
    type InitialState = Address;
    type Accumulator = FeeManagerState;
    type State = BlockState<Self::Accumulator>;

    async fn sync<A: SyncAccess + Send + Sync>(
        &self,
        fee_manager_address: &Address,
        block: &Block,
        access: &A,
    ) -> SyncResult<Self::Accumulator, A> {
        let contract: FeeManagerImpl<A> = access
            .build_sync_contract(*fee_manager_address, block.number, FeeManagerImpl::new)
            .await;

        let events = contract.fee_manager_created_filter().query().await.context(
            SyncContractError {
                err: "Error querying for fee manager created events",
            },
        )?;
        let created_event = events.first().unwrap();

        let events = contract.fee_redeemed_filter().query().await.context(
            SyncContractError {
                err: "Error querying for fee redeemed events",
            },
        )?;

        let mut validator_redeemed: [Option<(Address, U256)>; 8] = [None; 8];
        let mut validator_redeemed_sums: HashMap<Address, U256> = HashMap::new();

        for (index, ev) in events.iter().enumerate() {
            match validator_redeemed_sums.get(&ev.validator) {
                Some(amount) => validator_redeemed_sums[ev.validator] = amount + ev.amount,
                None => validator_redeemed_sums[ev.validator] = ev.amount,
            }
        }

        for (index, sum) in validator_redeemed_sums.iter().enumerate() {
            validator_redeemed[index] = Some((*sum.0, *sum.1));
        }

        let mut vouchers: HashMap<usize, HashMap<usize, HashMap<usize, bool>>> =
            HashMap::new();
        for ev in events {
            let (voucher_index, input_index, epoch_index) =
                convert_voucher_position_to_indices(ev.voucher_position);
            vouchers
                .entry(voucher_index)
                .or_insert_with(|| HashMap::new())
                .entry(input_index)
                .or_insert_with(|| HashMap::new())
                .entry(epoch_index)
                .or_insert_with(|| true);
        }

        Ok(FeeManagerState {
            validator_manager_address: created_event.validator_manager_cci,
            erc20_address: created_event.erc20,
            fee_per_claim: created_event.fee_per_claim,
            validator_redeemed,
            leftover_balance: I256::zero(),
            fee_manager_balance: U256::zero(),
        })
    }

    async fn fold<A: FoldAccess + Send + Sync>(
        &self,
        previous_state: &Self::Accumulator,
        block: &Block,
        access: &A,
    ) -> FoldResult<Self::Accumulator, A> {
        let voucher_address = previous_state.voucher_address;

        // If not in bloom copy previous state
        if !(fold_utils::contains_address(&block.logs_bloom, &voucher_address)
            && fold_utils::contains_topic(
                &block.logs_bloom,
                &VoucherExecutedFilter::signature(),
            ))
        {
            return Ok(previous_state.clone());
        }

        let contract = access
            .build_fold_contract(voucher_address, block.hash, VoucherImpl::new)
            .await;

        let events = contract.voucher_executed_filter().query().await.context(
            FoldContractError {
                err: "Error querying for voucher executed events",
            },
        )?;

        let mut vouchers = previous_state.vouchers.clone();
        for ev in events {
            let (voucher_index, input_index, epoch_index) =
                convert_voucher_position_to_indices(ev.voucher_position);
            vouchers
                .entry(voucher_index)
                .or_insert_with(|| HashMap::new())
                .entry(input_index)
                .or_insert_with(|| HashMap::new())
                .entry(epoch_index)
                .or_insert_with(|| true);
        }

        Ok(VoucherState {
            vouchers,
            voucher_address: voucher_address,
        })
    }

    fn convert(
        &self,
        accumulator: &BlockState<Self::Accumulator>,
    ) -> Self::State {
        accumulator.clone()
    }
}
