use crate::redirect::write_stdout;
use crate::executable::is_executable;


use std::sync::{LazyLock,Mutex};
use std::collections::HashMap;
use std::env;

use crate::command::JOBS;


pub static COMPLETIONS: LazyLock<Mutex<HashMap<String, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

//exit function 
pub fn exit_builtin(_args: &[&str], _op: Option<&str>, _file: Option<&str>) -> bool {
    false
}

//echo function 
pub fn echo_builtin(args: &[&str], op: Option<&str>, file: Option<&str>) -> bool {
    let output = format!("{}\n", args.join(" "));
    write_stdout(&output, op, file);
    true
}

//pwd function
pub fn pwd_builtin(_args: &[&str], op: Option<&str>, file: Option<&str>) -> bool {
    if let Ok(path) = env::current_dir() {
        let output = format!("{}\n", path.display());
        write_stdout(&output, op, file);
    } else {
        eprintln!("Error getting current directory");
    }
    true
}

//cd function 
pub fn cd_builtin(args: &[&str], _op: Option<&str>, _file: Option<&str>) -> bool {
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

//type builtin 
pub fn type_builtin(args: &[&str], op: Option<&str>, file: Option<&str>) -> bool {
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

//jobs builtin
pub fn jobs_builtin(
    _args: &[&str],
    _op: Option<&str>,
    _file: Option<&str>,
) -> bool {

    reap_jobs();

    let jobs = JOBS.lock().unwrap();
    let a = jobs.len();

    for (i, job) in jobs.iter().enumerate() {

        let marker = if i == a - 1 {
            '+'
        } else if i == a - 2 {
            '-'
        } else {
            ' '
        };

        println!(
            "[{}]{}  Running                 {}",
            job.job_num,
            marker,
            job.cmd
        );
    }

    true
}

pub fn reap_jobs() {
    let mut jobs = JOBS.lock().unwrap();

    if jobs.is_empty() {
        return;
    }

    let mut remove = Vec::new();
    let a = jobs.len();

    for (i, job) in jobs.iter_mut().enumerate() {
        let marker = if i == a - 1 {
            '+'
        } else if i == a - 2 {
            '-'
        } else {
            ' '
        };

        match job.child.try_wait() {
            Ok(Some(_)) => {
                let done = job.cmd.trim_end_matches(" &");
                println!(
                    "[{}]{}  Done                    {}",
                    job.job_num,
                    marker,
                    done
                );

                remove.push(i);
            }

            Ok(None) => {}

            Err(_) => {}
        }
    }

    for i in remove.into_iter().rev() {
        jobs.remove(i);
    }
}

//checks if the command is a builtin command
pub fn is_builtin_name(cmd: &str) -> bool {
    return matches!(cmd, "exit" | "echo" | "pwd" | "cd" | "type" | "complete" | "jobs");
}

//complete builtin
pub fn complete_builtin(args: &[&str], op: Option<&str>, file: Option<&str>) -> bool {
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
