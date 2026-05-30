use std::io::{self, Write};
use std::env;
use std::path::Path;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

fn is_builtin(cmd: &str) -> bool {
   return matches!(cmd, "exit" | "echo" | "type" | "pwd")
}

fn is_executable(cmd: &str) -> Option<String> {
    let path_var = env::var("PATH").unwrap_or_default();

    for dir in path_var.split(':') {
        let full_path = Path::new(dir).join(cmd);
        if full_path.exists() {
            if let Ok(metadata) = fs::metadata(&full_path) {
                let perms = metadata.permissions();

                //0o111 checks for execution permissions for user, group, and others --learned this.

                if perms.mode() & 0o111 != 0 {
                    return Some(full_path.to_string_lossy().to_string());
                }
            }
        }
    }
    None
}

// Vec<String> is a vector of strings [growing array]
fn parse_command(command: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();

    let mut in_single_quotes = false;
    let mut in_double_quotes = false;

    let mut chars = command.chars().peekable();

    while let Some(c) = chars.next() { //checking each character
        match c {

            '\\' if !in_single_quotes =>{
                // handle escape character by pushing the next character directly
                if let Some(next_char) = chars.next() {
                    current.push(next_char);
                }
            }

            '"' if !in_single_quotes => {
                // toggle quote mode by checking for "
                in_double_quotes = !in_double_quotes;
            }

            '\'' if !in_double_quotes => {
                // toggle quote mode by checking for '
                in_single_quotes = !in_single_quotes;
            }

            ' ' if !in_single_quotes && !in_double_quotes => {
                // argument separator
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
            }
             _ => { //default
                current.push(c);
            }
        }
    }

    if !current.is_empty() {
        args.push(current);
    }

    args
}

fn main() {
    loop{
        print!("$ ");
        io::stdout().flush().unwrap();
        let mut command = String::new();
        io::stdin().read_line(&mut command).unwrap();

        let parts = parse_command(command.trim());
        let parts_ref: Vec<&str> = parts
        .iter()
        .map(|s| s.as_str())
        .collect();

        //checking for > or 1>
        let redirect_pos = parts_ref
        .iter()
        .position(|&s| s == ">" || s == "1>");

        let mut output_file = None;
        let mut command_parts = &parts_ref[..];

        //separating the output file name and command part
        if let Some(pos) = redirect_pos {
            output_file = Some(parts_ref[pos + 1]);
            command_parts = &parts_ref[..pos];
        }

        //split the command into parts and match on the first part to determine the action

        match command_parts{

            [] => continue,
            ["exit"] => break,
            
            ["echo", args @ ..] => {
                let output = format!("{}\n", args.join(" "));

                if let Some(file) = output_file {
                    fs::write(file, output).unwrap();
                } else {
                    print!("{}", output);
                }
            },
            
            //pwd command
            ["pwd"] => {
                if let Ok(path) = env::current_dir() {
                    println!("{}", path.display());
                } else {
                    println!("Error getting current directory");
                }
            },

            //cd command
            ["cd", dir] => {
                let mut path = dir.to_string();

                if path == "~" {
                    path = env::var("HOME").unwrap();
                }

                if let Err(_) = env::set_current_dir(&path) {
                    println!("cd: {}: No such file or directory", dir);
                }
            },
            
            //type command 
            ["type", cmd] => {
                if is_builtin(cmd) {
                    println!("{} is a shell builtin ", cmd);
                    continue;
                }else if let Some(path) = is_executable(cmd) {
                    println!("{cmd} is {path}");
                } else {
                    println!("{cmd}: not found");
                }
            },

            ["type", args @ ..] => println!("{}: not found", args[0]),
            [cmd, args @ ..] => {
                if let Some(path) = is_executable(cmd) {

                    let mut command = Command::new(path);

                    command.arg0(cmd).args(args);

                    if let Some(file_name) = output_file {
                        let file = fs::File::create(file_name).unwrap();
                        command.stdout(Stdio::from(file));
                    }

                    command.status().unwrap();
                } else {
                    println!("{}: command not found", cmd);
                }
            }
        }
    }
}
