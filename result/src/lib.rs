use solana_instruction::error::InstructionError;

/// The overall result of the instruction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstructionResult {
    /// The number of compute units consumed by the instruction.
    pub compute_units_consumed: u32,
    /// The raw result of the program's execution.
    pub raw_result: Result<(), InstructionError>,
    /// The return data produced by the instruction, if any.
    pub return_data: Vec<u8>,
    pub messages: Vec<String>,
}
