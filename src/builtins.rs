use crate::redirect::write_stdout;
use crate::executable::is_executable;

use std::sync::{LazyLock,Mutex};
use std::collections::HashMap;
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};

use crate::command::JOBS;


use std::sync::{
    atomic::{AtomicUsize, Ordering},
};

pub static LAST_APPENDED: AtomicUsize = AtomicUsize::new(0);

pub static HISTORY: LazyLock<Mutex<Vec<String>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

pub static COMPLETIONS: LazyLock<Mutex<HashMap<String, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

//exit function 
pub fn exit_builtin(_args: &[&str], _op: Option<&str>, _file: Option<&str>, _out: &mut dyn Write) -> bool {
    false
}

//echo function 
pub fn echo_builtin(
    args: &[&str],
    op: Option<&str>,
    file: Option<&str>,
    out: &mut dyn Write,
) -> bool {
    let output = format!("{}\n", args.join(" "));

    if op.is_some() {
        write_stdout(&output, op, file);
    } else {
        let _ = write!(out, "{}", output);
    }

    true
}

//pwd function
pub fn pwd_builtin(_args: &[&str], op: Option<&str>, file: Option<&str>, out: &mut dyn Write) -> bool {
    if let Ok(path) = env::current_dir() {
        let output = format!("{}\n", path.display());
        if op.is_some() {
            write_stdout(&output, op, file);
        } else {
            let _ = write!(out, "{}", output);
        }
    } else {
        eprintln!("Error getting current directory");
    }
    true
}

//cd function 
pub fn cd_builtin(args: &[&str], _op: Option<&str>, _file: Option<&str>, _out: &mut dyn Write) -> bool {
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
pub fn type_builtin(args: &[&str], op: Option<&str>, file: Option<&str>, out: &mut dyn Write) -> bool {
    if args.is_empty() {
        return true;
    }
    let cmd = args[0];
    let output = if is_builtin_name(cmd) {
        format!("{} is a shell builtin\n", cmd)
    } else if let Some(path) = is_executable(cmd) {
        format!("{cmd} is {path}\n")
    } else {
        format!("{cmd}: not found\n")
    };

    if op.is_some() {
        write_stdout(&output, op, file);
    } else {
        let _ = write!(out, "{}", output);
    }
    true
}

//jobs builtin
pub fn jobs_builtin(_args: &[&str], op: Option<&str>, file: Option<&str>, out: &mut dyn Write) -> bool {
    let mut jobs = JOBS.lock().unwrap();
    let mut remove = Vec::new();
    let a = jobs.len();
    let mut output = String::new();
    
    for (i, job) in jobs.iter_mut().enumerate() {
        let marker = if i == a - 1 {'+'} else if i == a - 2 {'-'} else {' '};
        
        match job.child.try_wait() {
            Ok(None) => {
                output.push_str(&format!("[{}]{}  Running                 {}\n", job.job_num, marker, job.cmd));
            }

            Ok(Some(_)) => {
                let done = job.cmd.trim_end_matches(" &");
                output.push_str(&format!("[{}]{}  Done                 {}\n", job.job_num, marker, done));
                remove.push(i);
            }

            Err(_) => {}
        }
    }

    for i in remove.into_iter().rev() {
        jobs.remove(i);
    }

    if op.is_some() {
        write_stdout(&output, op, file);
    } else {
        let _ = write!(out, "{}", output);
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
    return matches!(cmd, "exit" | "echo" | "pwd" | "cd" | "type" | "complete" | "jobs"| "history");
}

//complete builtin
pub fn complete_builtin(args: &[&str], op: Option<&str>, file: Option<&str>, out: &mut dyn Write) -> bool {
    if args.len() >= 3 && args[0] == "-C" {
        let script = args[1]; // ignoring these warnings for now.
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

        let output = if let Some(script) = completions.get(command) {
            format!("complete -C '{}' {}\n", script, command)
        } else {
            format!("complete: {}: no completion specification\n", command)
        };

        if op.is_some() {
            write_stdout(&output, op, file);
        } else {
            let _ = write!(out, "{}", output);
        }
    }

    true
}


pub fn history_builtin(
    args: &[&str],
    _op: Option<&str>,
    _file: Option<&str>,
    out: &mut dyn Write,
) -> bool {

    // reading history
    if args.len() >= 2 && args[0] == "-r" {
        if let Ok(file) = File::open(args[1]) {
            let reader = BufReader::new(file);
            let mut history = HISTORY.lock().unwrap();

            for line in reader.lines() {
                if let Ok(cmd) = line {
                    if !cmd.trim().is_empty() {
                        history.push(cmd);
                    }
                }
            }
        }
        return true;
    }

    // writing history
    if args.len() >= 2 && args[0] == "-w" {
        if let Ok(mut file) = File::create(args[1]) {
            let history = HISTORY.lock().unwrap();

            for cmd in history.iter() {
                writeln!(file, "{}", cmd).unwrap();
            }
        }
        return true;
    }

    // appending history
    if args.len() >= 2 && args[0] == "-a" {

        let start = LAST_APPENDED.load(Ordering::SeqCst);

        if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(args[1])
        {
        let history = HISTORY.lock().unwrap();

        for cmd in history.iter().skip(start) {
            writeln!(file, "{}", cmd).unwrap();
        }

        LAST_APPENDED.store(history.len(), Ordering::SeqCst);
        }
        return true;
    }

    // history / history <n>
    let history = HISTORY.lock().unwrap();
    let len = history.len();

    let n = if !args.is_empty() {
        args[0].parse::<usize>().unwrap_or(len)
    } else {
        len
    };

    let start = len.saturating_sub(n);

    for (i, cmd) in history.iter().enumerate().skip(start) {
        writeln!(out, "{:>5}  {}", i + 1, cmd).unwrap();
    }

    true
}
