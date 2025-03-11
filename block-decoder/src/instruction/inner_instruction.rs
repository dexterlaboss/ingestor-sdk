
use {
    crate::{
        instruction::CompiledInstruction,
        errors::{
            conversion_error::ConversionError,
        }
    },
    serde_derive::{Deserialize, Serialize},
    solana_transaction_status_client_types::{
        UiCompiledInstruction,
        UiInnerInstructions,
        UiInstruction,
    },
    std::{
        convert::TryFrom,
    },
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InnerInstructions {
    /// Transaction instruction index
    pub index: u8,
    /// List of inner instructions
    pub instructions: Vec<InnerInstruction>,
}

impl TryFrom<UiInnerInstructions> for InnerInstructions {
    type Error = ConversionError;

    fn try_from(ui_inner_instructions: UiInnerInstructions) -> Result<Self, Self::Error> {
        let instructions_result: Result<Vec<_>, _> = ui_inner_instructions
            .instructions
            .into_iter()
            .map(|ix| match ix {
                UiInstruction::Compiled(ui_compiled) => Ok(InnerInstruction::from(ui_compiled)),
                _ => Err(ConversionError::UnsupportedInstructionFormat),
            })
            .collect();

        match instructions_result {
            Ok(instructions) => Ok(Self {
                index: ui_inner_instructions.index,
                instructions,
            }),
            Err(e) => Err(e),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InnerInstruction {
    /// Compiled instruction
    pub instruction: CompiledInstruction,
    /// Invocation stack height of the instruction,
    pub stack_height: Option<u32>,
}

impl From<UiCompiledInstruction> for InnerInstruction {
    fn from(ui_compiled_instruction: UiCompiledInstruction) -> Self {
        Self {
            instruction: CompiledInstruction::from(ui_compiled_instruction.clone()), // Clone is needed if CompiledInstruction::from consumes its argument
            stack_height: ui_compiled_instruction.stack_height,
        }
    }
}

impl From<InnerInstructions> for solana_transaction_status_client_types::InnerInstructions {
    fn from(instr: InnerInstructions) -> Self {
        Self {
            index: instr.index,
            instructions: instr.instructions.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<InnerInstruction> for solana_transaction_status_client_types::InnerInstruction {
    fn from(instr: InnerInstruction) -> Self {
        Self {
            instruction: instr.instruction.into(),
            stack_height: instr.stack_height,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use solana_transaction_status_client_types::{
        UiCompiledInstruction, UiInnerInstructions, UiInstruction, UiParsedInstruction
    };
    use crate::{instruction::CompiledInstruction, errors::conversion_error::ConversionError};

    #[test]
    fn test_ui_inner_instructions_to_inner_instructions_success() {
        let ui_compiled_instruction = UiCompiledInstruction {
            program_id_index: 1,
            accounts: vec![2, 3],
            data: "abcd".to_string(),
            stack_height: Some(5),
        };
        let ui_inner_instructions = UiInnerInstructions {
            index: 0,
            instructions: vec![UiInstruction::Compiled(ui_compiled_instruction.clone())],
        };

        let result = InnerInstructions::try_from(ui_inner_instructions);
        assert!(result.is_ok());
        let inner_instructions = result.unwrap();
        assert_eq!(inner_instructions.index, 0);
        assert_eq!(inner_instructions.instructions.len(), 1);
        assert_eq!(inner_instructions.instructions[0].stack_height, Some(5));
        assert_eq!(inner_instructions.instructions[0].instruction.program_id_index, 1);
    }

    #[test]
    fn test_ui_inner_instructions_to_inner_instructions_error() {
        use solana_transaction_status_client_types::{UiParsedInstruction, ParsedInstruction};

        let ui_inner_instructions = UiInnerInstructions {
            index: 1,
            instructions: vec![UiInstruction::Parsed(UiParsedInstruction::Parsed(ParsedInstruction {
                program: "Unsupported".to_string(),
                parsed: serde_json::Value::Null,
                program_id: "unsupported_program".to_string(),
                stack_height: None,
            }))],
        };

        let result = InnerInstructions::try_from(ui_inner_instructions);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConversionError::UnsupportedInstructionFormat));
    }

    #[test]
    fn test_ui_compiled_instruction_to_inner_instruction() {
        use bs58;

        let encoded_data = bs58::encode("data").into_string();

        let ui_compiled_instruction = UiCompiledInstruction {
            program_id_index: 2,
            accounts: vec![4, 5, 6],
            data: encoded_data,
            stack_height: Some(3),
        };

        let inner_instruction = InnerInstruction::from(ui_compiled_instruction.clone());

        assert_eq!(inner_instruction.stack_height, Some(3));
        assert_eq!(inner_instruction.instruction.program_id_index, 2);
        assert_eq!(inner_instruction.instruction.accounts, vec![4, 5, 6]);
        assert_eq!(inner_instruction.instruction.data, b"data".to_vec());
    }

    #[test]
    fn test_inner_instructions_to_solana_inner_instructions() {
        let compiled_instruction = CompiledInstruction {
            program_id_index: 3,
            accounts: vec![7, 8],
            data: vec![1, 2, 3],
        };
        let inner_instruction = InnerInstruction {
            instruction: compiled_instruction.clone(),
            stack_height: Some(10),
        };
        let inner_instructions = InnerInstructions {
            index: 2,
            instructions: vec![inner_instruction.clone()],
        };

        let solana_inner_instructions: solana_transaction_status_client_types::InnerInstructions = inner_instructions.into();
        assert_eq!(solana_inner_instructions.index, 2);
        assert_eq!(solana_inner_instructions.instructions.len(), 1);
        assert_eq!(solana_inner_instructions.instructions[0].stack_height, Some(10));
    }

    #[test]
    fn test_inner_instruction_to_solana_inner_instruction() {
        let compiled_instruction = CompiledInstruction {
            program_id_index: 4,
            accounts: vec![9, 10],
            data: vec![4, 5, 6],
        };
        let inner_instruction = InnerInstruction {
            instruction: compiled_instruction.clone(),
            stack_height: Some(7),
        };

        let solana_inner_instruction: solana_transaction_status_client_types::InnerInstruction = inner_instruction.into();
        assert_eq!(solana_inner_instruction.stack_height, Some(7));
        assert_eq!(solana_inner_instruction.instruction.program_id_index, 4);
    }
}



