use crate::executable::is_executable;
use crate::parser::parse_command;
use crate::builtins::*;
use std::collections::HashMap;

use std::fs;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio,Child};
use std::sync::{LazyLock,Mutex};
use std::io::{Read, Write};



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
        reap_jobs(); //reaping the jobs after executing a builtin command
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

                let job_num = jobs
                    .iter()
                    .map(|j| j.job_num)
                    .max()
                    .unwrap_or(0)
                    + 1;

                jobs.push(Jobs {
                    job_num,
                    child: process,
                    cmd: command,
                });

                println!("[{}] {}", job_num, pid);
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
    reap_jobs(); //reaping the jobs after executing a builtin command
    true
}

pub fn pipeline_execution(command: String) -> bool {
    use std::io::{Read, Write};

    let commands: Vec<&str> = command
        .split('|')
        .map(|s| s.trim())
        .collect();

    let mut previous_output: Option<Vec<u8>> = None;

    for (i, cmd_str) in commands.iter().enumerate() {
        let parts = parse_command(cmd_str);
        let parts_ref: Vec<&str> = parts.iter().map(|s| s.as_str()).collect();

        if parts_ref.is_empty() {
            continue;
        }

        let cmd = parts_ref[0];
        let args = &parts_ref[1..];

        let last = i == commands.len() - 1;

        //---------------------------------------------------
        // BUILTIN
        //---------------------------------------------------
        if BUILTINS.contains_key(cmd) {
            let mut output = Vec::<u8>::new();

            builtin_execution(
                cmd,
                args,
                None,
                None,
                &mut output,
            );

            if last {
                let _ = std::io::stdout().write_all(&output);
            } else {
                previous_output = Some(output);
            }

            continue;
        }

        //---------------------------------------------------
        // EXTERNAL
        //---------------------------------------------------
        let path = match is_executable(cmd) {
            Some(path) => path,
            None => {
                println!("{}: command not found", cmd);
                return true;
            }
        };

        let mut child = Command::new(path);

        child.arg0(cmd).args(args);

        if previous_output.is_some() {
            child.stdin(Stdio::piped());
        }

        if !last {
            child.stdout(Stdio::piped());
        }

        let mut child = child.spawn().unwrap();

        // Write the previous output to the stdin of the current command
        if let Some(buffer) = previous_output.take() {
            if let Some(stdin) = child.stdin.as_mut() {
                stdin.write_all(&buffer).unwrap();
            }
        }

        // last command
        if last {
            let _ = child.wait();
        } else {
            let mut buffer = Vec::new();

            child
                .stdout
                .take()
                .unwrap()
                .read_to_end(&mut buffer)
                .unwrap();

            let _ = child.wait();

            previous_output = Some(buffer);
        }
    }

    true
}

pub fn process_command(command: String, out: &mut dyn Write) -> bool {

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
