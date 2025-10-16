//! Core result types for SVM program execution.

use {
    solana_account::Account, solana_instruction::error::InstructionError,
    solana_program_error::ProgramError, solana_pubkey::Pubkey,
};

/// The result code of the program's execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProgramResult {
    /// The program executed successfully.
    Success,
    /// The program returned an error.
    Failure(ProgramError),
    /// Mollusk encountered an error while executing the program.
    UnknownError(InstructionError),
}

impl ProgramResult {
    /// Returns `true` if the program succeeded.
    pub fn is_ok(&self) -> bool {
        matches!(self, ProgramResult::Success)
    }

    /// Returns `true` if the program returned an error.
    pub fn is_err(&self) -> bool {
        !self.is_ok()
    }
}

impl From<Result<(), InstructionError>> for ProgramResult {
    fn from(result: Result<(), InstructionError>) -> Self {
        match result {
            Ok(()) => ProgramResult::Success,
            Err(err) => {
                if let Ok(program_error) = ProgramError::try_from(err.clone()) {
                    ProgramResult::Failure(program_error)
                } else {
                    ProgramResult::UnknownError(err)
                }
            }
        }
    }
}

/// The overall result of the instruction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstructionResult {
    /// The number of compute units consumed by the instruction.
    pub compute_units_consumed: u64,
    /// The result code of the program's execution.
    pub program_result: bool,
    /// The raw result of the program's execution.
    pub raw_result: Result<(), InstructionError>,
    /// The return data produced by the instruction, if any.
    pub return_data: Vec<u8>,
    /// The resulting accounts after executing the instruction.
    ///
    /// This includes all accounts provided to the processor, in the order
    /// they were provided. Any accounts that were modified will maintain
    /// their original position in this list, but with updated state.
    pub resulting_accounts: Vec<(Pubkey, Account)>,
}

impl Default for InstructionResult {
    fn default() -> Self {
        Self {
            compute_units_consumed: 0,
            program_result: true,
            raw_result: Ok(()),
            return_data: vec![],
            resulting_accounts: vec![],
        }
    }
}

impl InstructionResult {
    /// Get an account from the resulting accounts by its pubkey.
    pub fn get_account(&self, pubkey: &Pubkey) -> Option<&Account> {
        self.resulting_accounts
            .iter()
            .find(|(k, _)| k == pubkey)
            .map(|(_, a)| a)
    }

    pub fn absorb(&mut self, other: Self) {
        self.compute_units_consumed += other.compute_units_consumed;
        self.program_result = other.program_result;
        self.raw_result = other.raw_result;
        self.return_data = other.return_data;
        self.resulting_accounts = other.resulting_accounts;
    }
}
