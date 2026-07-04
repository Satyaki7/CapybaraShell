use crate::executable::is_executable;
use crate::parser::parse_command;
use crate::redirect::write_stdout;

use std::collections::HashMap;
use std::env;
use std::fs;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::{LazyLock, Mutex};

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
    m.insert("jobs", job_builtin);
    m
});

pub static COMPLETIONS: LazyLock<Mutex<HashMap<String, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn exit_builtin(_args: &[&str], _op: Option<&str>, _file: Option<&str>) -> bool {
    false
}

fn echo_builtin(args: &[&str], op: Option<&str>, file: Option<&str>) -> bool {
    let output = format!("{}\n", args.join(" "));
    write_stdout(&output, op, file);
    true
}

fn pwd_builtin(_args: &[&str], op: Option<&str>, file: Option<&str>) -> bool {
    if let Ok(path) = env::current_dir() {
        let output = format!("{}\n", path.display());
        write_stdout(&output, op, file);
    } else {
        eprintln!("Error getting current directory");
    }
    true
}

fn cd_builtin(args: &[&str], _op: Option<&str>, _file: Option<&str>) -> bool {
    let path_str = if args.is_empty() { "~" } else { args[0] };
    let mut path = path_str.to_string();

    if path == "~" {
        if let Ok(home) = env::var("HOME") {
            path = home;
        }
    }

    if let Err(_) = env::set_current_dir(&path) {
        println!("cd: {}: No such file or directory", path_str);
    }
    true
}

fn type_builtin(args: &[&str], op: Option<&str>, file: Option<&str>) -> bool {
    if args.is_empty() {
        return true;
    }
    let cmd = args[0];
    if is_builtin_name(cmd) {
        let output = format!("{} is a shell builtin\n", cmd);
        write_stdout(&output, op, file);
    } else if let Some(path) = is_executable(cmd) {
        let output = format!("{cmd} is {path}\n");
        write_stdout(&output, op, file);
    } else {
        let output = format!("{cmd}: not found\n");
        write_stdout(&output, op, file);
    }
    true
}

fn job_builtin(_args: &[&str], _op: Option<&str>, _file: Option<&str>) -> bool {
    true
}

//checks if the command is a builtin command
fn is_builtin_name(cmd: &str) -> bool {
    return matches!(cmd, "exit" | "echo" | "pwd" | "cd" | "type" | "complete" | "jobs");
}

fn complete_builtin(args: &[&str], op: Option<&str>, file: Option<&str>) -> bool {
    if args.len() >= 3 && args[0] == "-C" {
        let script = args[1];
        let command = args[2];

        COMPLETIONS
            .lock()
            .unwrap()
            .insert(command.to_string(), script.to_string());

        return true;
    }

    if args.len() >= 2 && args[0] == "-r" {
        let command = args[1];
        COMPLETIONS.lock().unwrap().remove(command);
        return true;
    }

    if args.len() >= 2 && args[0] == "-p" {
        let command = args[1];

        let completions = COMPLETIONS.lock().unwrap();

        if let Some(script) = completions.get(command) {
            let output = format!("complete -C '{}' {}\n", script, command);

            write_stdout(&output, op, file);
        } else {
            let output = format!("complete: {}: no completion specification\n", command);

            write_stdout(&output, op, file);
        }
    }

    true
}

pub fn execute(command: String) -> bool {
    let parts = parse_command(command.trim());
    let parts_ref: Vec<&str> = parts.iter().map(|s| s.as_str()).collect();

    if parts_ref.is_empty() {
        return true;
    }

    //checking for > or 1>
    let redirect_pos = parts_ref
        .iter()
        .position(|&s| s == ">" || s == "1>" || s == "2>" || s == ">>" || s == "1>>" || s == "2>>");

    let mut output_file = None;
    let mut command_parts = &parts_ref[..];
    let mut redirect_operator = None;

    //separating the output file name and command part
    if let Some(pos) = redirect_pos {
        //getting the output file name if it exists
        if pos + 1 < parts_ref.len() {
            output_file = Some(parts_ref[pos + 1]);
        }
        redirect_operator = Some(parts_ref[pos]);
        command_parts = &parts_ref[..pos];
    }

    if command_parts.is_empty() {
        return true;
    }

    let cmd = command_parts[0];
    let args = &command_parts[1..];

    //gets the builtin function for the command and executes it if it exists
    if let Some(builtin_fn) = BUILTINS.get(cmd) {
        return builtin_fn(args, redirect_operator, output_file);
    }

    // External command
    if let Some(path) = is_executable(cmd) {
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
        let _ = child.status();
    } else {
        println!("{}: command not found", cmd);
    }

    true
}
