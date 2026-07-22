use crate::executable::is_executable;
use crate::parser::parse_command;
use crate::builtins::*;
use std::collections::HashMap;

use std::fs;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio,Child};
use std::sync::{LazyLock,Mutex};
use std::io::{Write};



// A struct to represent a job in the shell
pub struct Jobs{
    pub job_num: usize,
    // pub status: String,
    // pub process_id: u32,
    pub child: Child,
    pub cmd: String,
}

pub static JOBS: LazyLock<Mutex<Vec<Jobs>>> =  LazyLock::new(|| Mutex::new(Vec::new()));



type BuiltinFn = fn(&[&str], Option<&str>, Option<&str>, &mut dyn Write) -> bool;

// A map of builtin command names to their corresponding functions
pub static BUILTINS: LazyLock<HashMap<&'static str, BuiltinFn>> = LazyLock::new(|| {
    let mut m: HashMap<&'static str, BuiltinFn> = HashMap::new();
    m.insert("exit", exit_builtin);
    m.insert("echo", echo_builtin);
    m.insert("pwd", pwd_builtin);
    m.insert("cd", cd_builtin);
    m.insert("type", type_builtin);
    m.insert("complete", complete_builtin);
    m.insert("jobs", jobs_builtin);
    m.insert("history", history_builtin);
    m
});

// getting builtin commands 
pub fn builtin_execution(
    cmd: &str,
    args: &[&str],
    redirect_operator: Option<&str>,
    output_file: Option<&str>,
    out: &mut dyn Write,
) -> bool {

    if let Some(builtin_fn) = BUILTINS.get(cmd) {
        let result = builtin_fn(args, redirect_operator, output_file, out);
        return result;
    }

    false
}

//external commands 
pub fn external_execution(
    command: String,
    cmd: &str,
    args: &[&str],
    redirect_operator: Option<&str>,
    output_file: Option<&str>,
    background: bool,
    _out: &mut dyn Write,
) -> bool {


    if let Some(path) = is_executable(cmd) { //gets the path of the command if it is an executable command

        let mut child = Command::new(path);
        child.arg0(cmd).args(args);

        if let Some(file_name) = output_file {
            let append = redirect_operator == Some(">>")
                || redirect_operator == Some("1>>")
                || redirect_operator == Some("2>>");

            let file_result = if append {
                fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(file_name)
            } else {
                fs::File::create(file_name)
            };

            if let Ok(file) = file_result {
                if redirect_operator == Some("2>") || redirect_operator == Some("2>>") {
                    child.stderr(Stdio::from(file));
                } else {
                    child.stdout(Stdio::from(file));
                }
            }
        }
        // let _ = child.status();
       match child.spawn() {
        Ok(process) => {
            if background {
                let pid = process.id();

                let mut jobs = JOBS.lock().unwrap();

                let mut job_num = 1;
                while jobs.iter().any(|j| j.job_num == job_num) {
                    job_num += 1;
                }

                jobs.push(Jobs {
                    job_num,
                    child: process,
                    cmd: command,
                });

                let _ = writeln!(_out, "[{}] {}", job_num, pid);
                let _ = _out.flush();
            } else {
                let mut process = process;
                let _ = process.wait();
            }
        }
        Err(_) => { /* ... */ }
    }
    } else {
        println!("{}: command not found", cmd);
    }
    true
}

enum PipelineStep {
    Builtin {
        cmd: String,
        args: Vec<String>,
    },
    External {
        cmd: String,
        args: Vec<String>,
    },
}

fn run_external_pipeline_segment(
    ext_steps: &[PipelineStep],
    mut previous_output: Option<Vec<u8>>,
    is_last_segment: bool,
) -> Option<Vec<u8>> {
    use std::io::{Read, Write};

    let mut children: Vec<Child> = Vec::new();
    let mut last_stdout: Option<std::process::ChildStdout> = None;

    for (idx, step) in ext_steps.iter().enumerate() {
        let (cmd, args) = match step {
            PipelineStep::External { cmd, args } => (cmd, args),
            _ => unreachable!(),
        };

        let path = match is_executable(cmd) {
            Some(path) => path,
            None => {
                println!("{}: command not found", cmd);
                return None;
            }
        };

        let mut command = Command::new(path);
        command.arg0(cmd).args(args);

        // Configure stdin
        if idx == 0 {
            if previous_output.is_some() {
                command.stdin(Stdio::piped());
            }
        } else {
            if let Some(stdout) = last_stdout.take() {
                command.stdin(Stdio::from(stdout));
            }
        }

        // Configure stdout
        let is_last_step_in_segment = idx == ext_steps.len() - 1;
        if is_last_step_in_segment {
            if is_last_segment {
                // Inherit standard stdout
            } else {
                command.stdout(Stdio::piped());
            }
        } else {
            command.stdout(Stdio::piped());
        }

        match command.spawn() {
            Ok(mut child) => {
                if idx == 0 {
                    if let Some(buffer) = previous_output.take() {
                        if let Some(mut stdin) = child.stdin.take() {
                            std::thread::spawn(move || {
                                let _ = stdin.write_all(&buffer);
                            });
                        }
                    }
                }

                if !is_last_step_in_segment {
                    last_stdout = child.stdout.take();
                } else if !is_last_segment {
                    last_stdout = child.stdout.take();
                }

                children.push(child);
            }
            Err(_) => {
                println!("{}: command not found", cmd);
                return None;
            }
        }
    }

    let mut output_buffer = None;
    if !is_last_segment {
        if let Some(mut stdout) = last_stdout {
            let mut buffer = Vec::new();
            let _ = stdout.read_to_end(&mut buffer);
            output_buffer = Some(buffer);
        }
    }

    // Wait for the last child in this segment to finish
    if let Some(mut last_child) = children.pop() {
        let _ = last_child.wait();
    }

    // Reap other children asynchronously to avoid blocking
    for mut child in children {
        std::thread::spawn(move || {
            let _ = child.wait();
        });
    }

    output_buffer
}

pub fn pipeline_execution(command: String) -> bool {
    use std::io::Write;

    let commands: Vec<&str> = command
        .split('|')
        .map(|s| s.trim())
        .collect();

    let mut steps = Vec::new();
    for cmd_str in commands {
        let parts = parse_command(cmd_str);
        if parts.is_empty() {
            continue;
        }
        let cmd = parts[0].clone();
        let args: Vec<String> = parts[1..].iter().cloned().collect();
        if BUILTINS.contains_key(cmd.as_str()) {
            steps.push(PipelineStep::Builtin { cmd, args });
        } else {
            steps.push(PipelineStep::External { cmd, args });
        }
    }

    let mut previous_output: Option<Vec<u8>> = None;
    let mut i = 0;

    while i < steps.len() {
        match &steps[i] {
            PipelineStep::Builtin { cmd, args } => {
                let last = i == steps.len() - 1;
                let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                let mut output = Vec::new();
                builtin_execution(
                    cmd,
                    &args_ref,
                    None,
                    None,
                    &mut output,
                );
                if last {
                    let _ = std::io::stdout().write_all(&output);
                } else {
                    previous_output = Some(output);
                }
                i += 1;
            }
            PipelineStep::External { .. } => {
                let mut j = i;
                while j < steps.len() {
                    if let PipelineStep::External { .. } = &steps[j] {
                        j += 1;
                    } else {
                        break;
                    }
                }
                let ext_steps = &steps[i..j];
                let is_last_segment = j == steps.len();

                previous_output = run_external_pipeline_segment(ext_steps, previous_output, is_last_segment);

                i = j;
            }
        }
    }

    true
}

pub fn process_command(command: String, out: &mut dyn Write) -> bool {
    reap_jobs(out);

    if command.contains('|') {
        return pipeline_execution(command);
    }
    
    let parts = parse_command(command.trim()); //returns a vector of strings from the line entered
    let parts_ref: Vec<&str> = parts.iter().map(|s| s.as_str()).collect();


    if parts_ref.is_empty() {
        return true;
    }

    //checking for > or 1>
    let redirect_pos = parts_ref
        .iter()
        .position(|&s| s == ">" || s == "1>" || s == "2>" || s == ">>" || s == "1>>" || s == "2>>");

    let mut output_file = None;
    let mut command_parts = parts_ref.clone(); //default command parts is the whole command, will be changed if there is a redirect operator
    
    let mut redirect_operator = None;

    //separating the output file name and command part
    if let Some(pos) = redirect_pos {
        //getting the output file name if it exists
        if pos + 1 < parts_ref.len() {
            output_file = Some(parts_ref[pos + 1]);
        }
        redirect_operator = Some(parts_ref[pos]);
        command_parts = parts_ref[..pos].to_vec();
    }

    if command_parts.is_empty() {
        return true;
    }

    // checking for & to see if it is a background process and removing the '&'
    let background = if command_parts.last().map_or(false, |&arg| arg == "&") {
    command_parts.pop(); 
    true
    } else {
        false
    };

    let cmd = command_parts[0]; //getting the command name 
    let args = &command_parts[1..]; //getting the command arguments

   if BUILTINS.contains_key(cmd) {
    return builtin_execution(
        cmd,
        args,
        redirect_operator,
        output_file,
        out,
        );
    }

    return external_execution(
         command,
         cmd,
         args,
         redirect_operator,
         output_file,
         background,
         out,
     );
    
}
