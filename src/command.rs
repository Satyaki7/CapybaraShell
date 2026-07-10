use crate::executable::is_executable;
use crate::parser::parse_command;
use crate::builtins::*;
use std::collections::HashMap;

use std::fs;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio,Child};
use std::sync::{LazyLock,Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_JOB: AtomicUsize = AtomicUsize::new(1);


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


pub fn execute(command: String) -> bool {

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
        Ok(mut process) => {
            if background {
                let job = NEXT_JOB.fetch_add(1, Ordering::SeqCst); //increasing the job count 
                let pid = process.id();
                JOBS.lock().unwrap().push(Jobs {
                    job_num: job,
                    child: process,
                    cmd: command,
                });
                println!("[{}] {}", job, pid); // spawning a background process, printing its PID
            } else {
                let _ = process.wait();
            }
        }
        Err(e) => {
            eprintln!("{}", e);
        }
    }
    } else {
        println!("{}: command not found", cmd);
    }
    reap_jobs(); //reaping the jobs after executing a builtin command
    true
}
