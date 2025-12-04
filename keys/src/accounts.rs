//! Instruction <-> Transaction account compilation.

use {
    crate::keys::KeyMap,
    ahash::HashMap,
    mollusk_svm_error::error::{MolluskError, MolluskPanic},
    solana_account::{Account, AccountSharedData},
    solana_instruction::Instruction,
    solana_pubkey::Pubkey,
    solana_transaction_context::{IndexOfAccount, InstructionAccount},
};

// Helper struct to avoid cloning instruction data.
pub struct CompiledInstructionWithoutData {
    pub program_id_index: u8,
    pub accounts: Vec<u8>,
}

pub fn compile_instruction_without_data(
    key_map: &KeyMap,
    instruction: &Instruction,
) -> CompiledInstructionWithoutData {
    let program_id_index = key_map
        .position(&instruction.program_id)
        .or_panic_with(MolluskError::ProgramIdNotMapped(&instruction.program_id));

    let program_id_index = u8::try_from(program_id_index)
        .or_panic_with(MolluskError::AccountIndexOverflow(program_id_index));

    let accounts: Vec<u8> = instruction
        .accounts
        .iter()
        .map(|account_meta| {
            let index = key_map
                .position(&account_meta.pubkey)
                .or_panic_with(MolluskError::AccountMissing(&account_meta.pubkey));

            u8::try_from(index).or_panic_with(MolluskError::AccountIndexOverflow(index))
        })
        .collect();

    CompiledInstructionWithoutData {
        program_id_index,
        accounts,
    }
}

pub fn compile_instruction_accounts(
    key_map: &KeyMap,
    compiled_instruction: &CompiledInstructionWithoutData,
) -> Vec<InstructionAccount> {
    compiled_instruction
        .accounts
        .iter()
        .map(|&index_in_transaction| {
            let index_in_transaction = index_in_transaction as usize;
            InstructionAccount::new(
                index_in_transaction as IndexOfAccount,
                key_map.is_signer_at_index(index_in_transaction),
                key_map.is_writable_at_index(index_in_transaction),
            )
        })
        .collect()
}

pub fn compile_transaction_accounts_for_instruction(
    key_map: &KeyMap,
    instruction: &Instruction,
    accounts: Vec<(Pubkey, Account)>,
    loader_key: Pubkey,
) -> Vec<(Pubkey, AccountSharedData)> {
    let mut accounts = accounts.into_iter().collect::<HashMap<_, _>>();

    key_map
        .keys()
        .map(|key| {
            if key == &instruction.program_id {
                (
                    instruction.program_id,
                    Account {
                        lamports: 0,
                        data: Vec::new(),
                        owner: loader_key,
                        executable: true,
                        rent_epoch: 0,
                    }
                    .into(),
                )
            } else {
                accounts
                    .remove_entry(key)
                    .map(|(key, acc)| (key, acc.into()))
                    .or_panic_with(MolluskError::AccountMissing(key))
            }
        })
        .collect()
}
