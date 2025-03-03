
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


