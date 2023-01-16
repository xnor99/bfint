use std::{env, fs, io};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

#[cfg_attr(test, derive(Debug, PartialEq))]
enum BFInstruction {
    Add(u8),
    Subtract(u8),
    // TODO: replace Subtract variant with Add
    IncrementPointer(usize),
    DecrementPointer(usize),
    // TODO: replace DecrementPointer variant with IncrementPointer
    Output,
    Input,
    LoopStart(usize),
    LoopEnd(usize),
}

fn parse_data(data: &[u8]) -> Option<Vec<BFInstruction>> {
    let mut instructions = Vec::new();
    let mut loop_stack = Vec::new();
    let mut last_instruction = None;
    for &byte in data {
        match byte {
            b'+' => match last_instruction.take() {
                Some(BFInstruction::Add(val)) => last_instruction = Some(BFInstruction::Add(val.wrapping_add(1))),
                Some(other_instruction) => {
                    instructions.push(Some(other_instruction));
                    last_instruction = Some(BFInstruction::Add(1));
                }
                None => last_instruction = Some(BFInstruction::Add(1))
            }
            b'-' => match last_instruction.take() {
                Some(BFInstruction::Subtract(val)) => last_instruction = Some(BFInstruction::Subtract(val.wrapping_add(1))),
                Some(other_instruction) => {
                    instructions.push(Some(other_instruction));
                    last_instruction = Some(BFInstruction::Subtract(1));
                }
                None => last_instruction = Some(BFInstruction::Subtract(1))
            }
            b'>' => match last_instruction.take() {
                Some(BFInstruction::IncrementPointer(by)) => last_instruction = Some(BFInstruction::IncrementPointer(by.wrapping_add(1))),
                Some(other_instruction) => {
                    instructions.push(Some(other_instruction));
                    last_instruction = Some(BFInstruction::IncrementPointer(1));
                }
                None => last_instruction = Some(BFInstruction::IncrementPointer(1))
            }
            b'<' => match last_instruction.take() {
                Some(BFInstruction::DecrementPointer(by)) => last_instruction = Some(BFInstruction::DecrementPointer(by.wrapping_add(1))),
                Some(other_instruction) => {
                    instructions.push(Some(other_instruction));
                    last_instruction = Some(BFInstruction::DecrementPointer(1));
                }
                None => last_instruction = Some(BFInstruction::DecrementPointer(1))
            }
            b'.' => {
                if let Some(last) = last_instruction.take() {
                    instructions.push(Some(last));
                }
                instructions.push(Some(BFInstruction::Output));
            }
            b',' => {
                if let Some(last) = last_instruction.take() {
                    instructions.push(Some(last));
                }
                instructions.push(Some(BFInstruction::Input));
            }
            b'[' => {
                if let Some(last) = last_instruction.take() {
                    instructions.push(Some(last));
                }
                loop_stack.push(instructions.len());
                instructions.push(None);
            }
            b']' => {
                if let Some(last) = last_instruction.take() {
                    instructions.push(Some(last));
                }
                let loop_start_idx = loop_stack.pop()?;
                instructions[loop_start_idx] = Some(BFInstruction::LoopStart(instructions.len()));
                instructions.push(Some(BFInstruction::LoopEnd(loop_start_idx)));
            }
            _ => {}
        }
    }
    
    if let Some(last_instruction) = last_instruction {
        instructions.push(Some(last_instruction));
    }
    
    let mut instructions_return = Vec::with_capacity(instructions.len());
    for instruction in instructions {
        instructions_return.push(instruction?);
    }
    
    Some(instructions_return)
}

struct Args {
    path: PathBuf,
    memory_size: usize,
}

fn parse_args(args: impl Iterator<Item=String>) -> Option<Args> {
    let mut args = args.skip(1);
    Some(Args {
        path: PathBuf::from(args.next()?),
        memory_size: args.next()?.parse().ok()?,
    })
}

#[cfg_attr(test, derive(Debug, PartialEq))]
enum ExecutionResult {
    Ok,
    MemoryAccessError,
    IOError,
}

fn run_program(program: &[BFInstruction], memory_size: usize) -> ExecutionResult {
    let mut program_counter = 0;
    let mut data_pointer = 0;
    let mut memory = vec![0u8; memory_size];
    let mut stdout = io::stdout().lock();
    let mut stdin = io::stdin().lock();
    while program_counter < program.len() {
        match program[program_counter] {
            BFInstruction::Add(val) => {
                let Some(current_byte) = memory.get_mut(data_pointer) else {
                    return ExecutionResult::MemoryAccessError;
                };
                
                *current_byte = current_byte.wrapping_add(val);
            }
            BFInstruction::Subtract(val) => {
                let Some(current_byte) = memory.get_mut(data_pointer) else {
                    return ExecutionResult::MemoryAccessError;
                };
                
                *current_byte = current_byte.wrapping_sub(val);
            }
            BFInstruction::IncrementPointer(by) => data_pointer = data_pointer.wrapping_add(by),
            BFInstruction::DecrementPointer(by) => data_pointer = data_pointer.wrapping_sub(by),
            BFInstruction::Output => {
                let Some(&current_byte) = memory.get(data_pointer) else {
                    return ExecutionResult::MemoryAccessError;
                };
                
                if stdout.write(&[current_byte]).is_err() || stdout.flush().is_err() {
                    return ExecutionResult::IOError;
                }
            }
            BFInstruction::Input => {
                let Some(current_byte) = memory.get_mut(data_pointer) else {
                    return ExecutionResult::MemoryAccessError;
                };
                
                let mut read_byte = [0; 1];
                match stdin.read(&mut read_byte) {
                    Ok(0) => *current_byte = 0,
                    Ok(_) => *current_byte = read_byte[0],
                    Err(_) => return ExecutionResult::IOError
                }
            }
            BFInstruction::LoopStart(idx) => {
                let Some(&current_byte) = memory.get(data_pointer) else {
                    return ExecutionResult::MemoryAccessError;
                };
                
                if current_byte == 0 {
                    program_counter = idx;
                }
            }
            BFInstruction::LoopEnd(idx) => {
                let Some(&current_byte) = memory.get(data_pointer) else {
                    return ExecutionResult::MemoryAccessError;
                };
                
                if current_byte != 0 {
                    program_counter = idx;
                }
            }
        }
        program_counter += 1;
    }
    ExecutionResult::Ok
}

fn main() -> ExitCode {
    let Some(Args { path, memory_size }) = parse_args(env::args()) else {
        eprintln!("usage: [path] [mem_size]");
        return ExitCode::FAILURE;
    };
    
    let Ok(file_contents) = fs::read(path) else {
        eprintln!("couldn't read file");
        return ExitCode::FAILURE;
    };
    
    let Some(program) = parse_data(&file_contents) else {
        eprintln!("couldn't parse program");
        return ExitCode::FAILURE;
    };
    
    match run_program(&program, memory_size) {
        ExecutionResult::Ok => ExitCode::SUCCESS,
        ExecutionResult::MemoryAccessError => {
            eprintln!("memory access error");
            ExitCode::FAILURE
        }
        ExecutionResult::IOError => {
            eprintln!("I/O error");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn invalid_memory_access() {
        assert_eq!(run_program(&parse_data(b">+").unwrap(), 1), ExecutionResult::MemoryAccessError);
        assert_eq!(run_program(&parse_data(b"<+").unwrap(), 1), ExecutionResult::MemoryAccessError);
        assert_eq!(run_program(&parse_data(b"<>+").unwrap(), 1), ExecutionResult::Ok);
        assert_eq!(run_program(&parse_data(b">[]").unwrap(), 1), ExecutionResult::MemoryAccessError);
    }
    
    #[test]
    fn optimizations() {
        assert_eq!(parse_data(b"++++++.---,").unwrap(), [BFInstruction::Add(6), BFInstruction::Output, BFInstruction::Subtract(3), BFInstruction::Input]);
    }
    
    #[test]
    fn invalid_loops() {
        assert!(parse_data(b"][").is_none());
        assert!(parse_data(b"[[]").is_none());
        assert!(parse_data(b"[]]").is_none());
    }
}
