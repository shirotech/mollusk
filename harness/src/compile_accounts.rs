//! Instruction <-> Transaction account compilation, with key deduplication,
//! privilege handling, and program account stubbing.

use {
    mollusk_svm_keys::{
        accounts::{
            compile_instruction_accounts, compile_instruction_without_data,
            compile_transaction_accounts_for_instruction,
        },
        keys::KeyMap,
    },
    solana_account::{Account, AccountSharedData},
    solana_instruction::Instruction,
    solana_pubkey::Pubkey,
    solana_transaction_context::InstructionAccount,
};

pub struct CompiledAccounts {
    pub program_id_index: u8,
    pub instruction_accounts: Vec<InstructionAccount>,
    pub transaction_accounts: Vec<(Pubkey, AccountSharedData)>,
}

pub fn compile_accounts(
    instruction: &Instruction,
    accounts: Vec<(Pubkey, Account)>,
    loader_key: Pubkey,
) -> CompiledAccounts {
    let key_map = KeyMap::compile_from_instruction(instruction);
    let compiled_instruction = compile_instruction_without_data(&key_map, instruction);
    let instruction_accounts = compile_instruction_accounts(&key_map, &compiled_instruction);
    let transaction_accounts =
        compile_transaction_accounts_for_instruction(&key_map, instruction, accounts, loader_key);

    CompiledAccounts {
        program_id_index: compiled_instruction.program_id_index,
        instruction_accounts,
        transaction_accounts,
    }
}
