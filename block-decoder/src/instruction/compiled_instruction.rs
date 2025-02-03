use {
    serde_derive::{Deserialize, Serialize},
    solana_program::short_vec,
    solana_transaction_status::{
        UiCompiledInstruction,
    },
};

/// A compact encoding of an instruction.
///
/// A `CompiledInstruction` is a component of a multi-instruction [`Message`],
/// which is the core of a Solana transaction. It is created during the
/// construction of `Message`. Most users will not interact with it directly.
///
/// [`Message`]: crate::message::Message
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CompiledInstruction {
    /// Index into the transaction keys array indicating the program account that executes this instruction.
    pub program_id_index: u8,
    /// Ordered indices into the transaction keys array indicating which accounts to pass to the program.
    #[serde(with = "short_vec")]
    pub accounts: Vec<u8>,
    /// The program input data.
    #[serde(with = "short_vec")]
    pub data: Vec<u8>,
}

impl From<UiCompiledInstruction> for CompiledInstruction {
    fn from(ui_compiled_instruction: UiCompiledInstruction) -> Self {
        Self {
            program_id_index: ui_compiled_instruction.program_id_index,
            accounts: ui_compiled_instruction.accounts,
            data: bs58::decode(ui_compiled_instruction.data).into_vec().unwrap(),
        }
    }
}

impl From<CompiledInstruction> for solana_sdk::instruction::CompiledInstruction {
    fn from(instr: CompiledInstruction) -> Self {
        Self {
            program_id_index: instr.program_id_index,
            accounts: instr.accounts,
            data: instr.data,
        }
    }
}
