use crate::evm::{block::Block, context::Context};
use crate::evm::eval::eval;
use crate::evm::jump_map::JumpMap;
use crate::evm::memory::Memory;
use crate::evm::stack::Stack;
use primitive_types::{U256, H160};

pub enum ControlFlow {
    Continue(usize),
    Jump(usize),
    Exit(ExitReason),
}

pub enum ExitReason {
    Error(EvmError),
    Success(ExitSuccess),
}

pub enum ExitSuccess {
    Stop,
    Return(Vec<u8>),
}

#[derive(Debug, Copy, Clone)]
pub enum EvmError {
    StackUnderflow,
    InvalidInstruction,
    InvalidJump,
    Revert(U256),
    OpcodeNotStatic(u8),
}

enum EvmStatus {
    Running,
    Exited(ExitReason),
}

pub struct EvmResult {
    pub stack: Vec<U256>,
    pub success: bool,
    pub error: Option<EvmError>,
    pub logs: Vec<Log>,
    pub return_val: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct Log {
    pub address: String,
    pub data: String,
    pub topics: Vec<String>,
}

impl Log {
    pub fn new(address: H160, data: &[u8]) -> Self {
        Self {
            address: address.to_string(),
            data: hex::encode(data),
            topics: Vec::new(),
        }
    }

    pub fn add_topic(&mut self, topic: U256) {
        let mut bytes: [u8; 32] = [0; 32];
        topic.to_big_endian(&mut bytes);

        let topic_string = hex::encode(bytes);

        // TODO: check if this can be done using H160 or similar, maybe H256
        let mut prepended_topic: String = "0x".to_owned();
        prepended_topic.push_str(&topic_string);

        self.topics.push(prepended_topic);
    }
}

pub struct Machine<'a> {
    pub stack: Stack,
    pub memory: Memory,
    pub return_data_buffer: Vec<u8>,
    pub context: Context<'a>,
    pub jump_map: JumpMap,
    pub code: &'a [u8],
    pub logs: Vec<Log>,
    pub pc: usize,
}

impl<'a> Machine<'a> {
    pub fn new(
        code: &'a [u8],
        context: Context<'a>,
    ) -> Self {
        Self {
            stack: Stack::new(),
            memory: Memory::new(),
            jump_map: JumpMap::new(code),
            return_data_buffer: Vec::new(),
            logs: Vec::new(),
            context,
            code,
            pc: 0,
        }
    }

    fn stack(&self) -> Vec<U256> {
        self.stack.data()
    }

    pub fn opcode(&self) -> u8 {
        self.code[self.pc]
    }

    fn step(&mut self) -> EvmStatus {
        match eval(self) {
            ControlFlow::Continue(steps) => {
                self.pc += steps;
                EvmStatus::Running
            }
            ControlFlow::Jump(position) => {
                self.pc = position;
                EvmStatus::Running
            }
            ControlFlow::Exit(reason) => EvmStatus::Exited(reason),
        }
    }

    pub fn execute(&mut self) -> EvmResult {
        while self.pc < self.code.len() {
            match self.step() {
                EvmStatus::Running => continue,
                EvmStatus::Exited(reason) => match reason {
                    ExitReason::Success(success) => match success {
                        ExitSuccess::Stop => break,
                        ExitSuccess::Return(val) => {
                            return EvmResult {
                                stack: self.stack(),
                                success: true,
                                error: None,
                                logs: self.logs.clone(),
                                return_val: Some(val),
                            }
                        }
                    },
                    ExitReason::Error(error) => {
                        return EvmResult {
                            stack: self.stack(),
                            success: false,
                            error: Some(error),
                            logs: self.logs.clone(),
                            return_val: None,
                        }
                    }
                },
            }
        }

        EvmResult {
            stack: self.stack(),
            success: true,
            error: None,
            logs: self.logs.clone(),
            return_val: None,
        }
    }
}