use crate::executable::is_executable;
use crate::parser::parse_command;
use crate::builtins::*;
use std::collections::HashMap;

use std::fs;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio,Child};
use std::sync::{LazyLock,Mutex};



// A struct to represent a job in the shell
pub struct Jobs{
    pub job_num: usize,
    // pub status: String,
    // pub process_id: u32,
    pub child: Child,
    pub cmd: String,
}

pub static JOBS: LazyLock<Mutex<Vec<Jobs>>> =  LazyLock::new(|| Mutex::new(Vec::new()));



type BuiltinFn = fn(&[&str], Option<&str>, Option<&str>) -> bool;

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

pub fn pipeline_execution(command: String) -> bool{
    let (left, right) = command.split_once('|').unwrap();

    //parsing the commands on the left and right of the pipe
    let left_parts = parse_command(left.trim());
    let right_parts = parse_command(right.trim());

    // separating the command and arguments for the left and right commands
    let left_cmd = &left_parts[0];
    let left_args = &left_parts[1..];
    let right_cmd = &right_parts[0];
    let right_args = &right_parts[1..];

    //finding the path if it is an executable command.
    let left_path = match is_executable(left_cmd) {
        Some(path) => path,
        None => {
            println!("{}: command not found", left_cmd);
            return true;
        }
    };

    let right_path = match is_executable(right_cmd) {
        Some(path) => path,
        None => {
            println!("{}: command not found", right_cmd);
            return true;
        }
    };

    //spawing the first command.
    let mut first = Command::new(left_path);
    first.arg0(left_cmd)
        .args(left_args)
        .stdout(Stdio::piped());

    let mut first = first.spawn().unwrap();

    let stdout = first.stdout.take().unwrap();
    let mut second = Command::new(right_path);
    second.arg0(right_cmd)
        .args(right_args)
        .stdin(Stdio::from(stdout));

    let mut second = second.spawn().unwrap();

    let _ = first.wait();
    let _ = second.wait();

    return true
}


pub fn execute(command: String) -> bool {

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

    //gets the builtin function for the command and executes it if it exists
    if let Some(builtin_fn) = BUILTINS.get(cmd) {
        if cmd == "jobs"{
            return builtin_fn(args, redirect_operator, output_file);
        }
        let result = builtin_fn(args, redirect_operator, output_file);
        reap_jobs(); //reaping the jobs after executing a builtin command
        return result;
    }

    // External command 
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
