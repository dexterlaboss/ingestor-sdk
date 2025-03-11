use {
    serde_derive::{Deserialize, Serialize},
    solana_short_vec as short_vec,
    solana_transaction_status_client_types::{
        UiCompiledInstruction,
    }
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

impl From<CompiledInstruction> for solana_message::compiled_instruction::CompiledInstruction {
    fn from(instr: CompiledInstruction) -> Self {
        Self {
            program_id_index: instr.program_id_index,
            accounts: instr.accounts,
            data: instr.data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;
    use solana_transaction_status_client_types::UiCompiledInstruction;
    use solana_message::compiled_instruction::CompiledInstruction as SolanaCompiledInstruction;
    use bs58;

    #[test]
    fn test_serialization_deserialization() {
        let instr = CompiledInstruction {
            program_id_index: 1,
            accounts: vec![0, 1, 2],
            data: vec![10, 20, 30],
        };

        let json = serde_json::to_string(&instr).unwrap();
        let deserialized: CompiledInstruction = serde_json::from_str(&json).unwrap();

        assert_eq!(instr, deserialized);
    }

    #[test]
    fn test_from_ui_compiled_instruction_valid() {
        let ui_instr = UiCompiledInstruction {
            program_id_index: 2,
            accounts: vec![3, 4, 5],
            data: bs58::encode(vec![100, 101, 102]).into_string(),
            stack_height: None,
        };

        let compiled_instr: CompiledInstruction = ui_instr.clone().into();

        assert_eq!(compiled_instr.program_id_index, ui_instr.program_id_index);
        assert_eq!(compiled_instr.accounts, ui_instr.accounts);
        assert_eq!(compiled_instr.data, vec![100, 101, 102]);
    }

    #[test]
    fn test_from_ui_compiled_instruction_invalid_base58() {
        let ui_instr = UiCompiledInstruction {
            program_id_index: 0,
            accounts: vec![0],
            data: "invalid_base58".to_string(),
            stack_height: None,
        };

        let result = std::panic::catch_unwind(|| {
            let _compiled_instr: CompiledInstruction = ui_instr.into();
        });

        assert!(result.is_err());
    }

    #[test]
    fn test_from_compiled_instruction_to_solana_compiled_instruction() {
        let instr = CompiledInstruction {
            program_id_index: 3,
            accounts: vec![6, 7, 8],
            data: vec![9, 10, 11],
        };

        let solana_instr: SolanaCompiledInstruction = instr.clone().into();

        assert_eq!(solana_instr.program_id_index, instr.program_id_index);
        assert_eq!(solana_instr.accounts, instr.accounts);
        assert_eq!(solana_instr.data, instr.data);
    }
}