mod parser;
mod executable;
mod redirect;

use parser::parse_command;
use executable::is_executable;
use redirect::write_stdout;

use std::io::{self, Write};
use std::env;
use std::fs;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};



fn is_builtin(cmd: &str) -> bool {
   return matches!(cmd, "exit" | "echo" | "type" | "pwd")
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
        .position(|&s| s == ">" || s == "1>"|| s == "2>" || s == ">>" || s == "1>>" || s == "2>>");

        let mut output_file = None;
        let mut command_parts = &parts_ref[..];
        let mut redirect_operator = None;

        //separating the output file name and command part
        if let Some(pos) = redirect_pos {
            output_file = Some(parts_ref[pos + 1]);
            redirect_operator = Some(parts_ref[pos]);
            command_parts = &parts_ref[..pos];
        }

        //split the command into parts and match on the first part to determine the action

        match command_parts{

            [] => continue,
            ["exit"] => break,
            
            ["echo", args @ ..] => {
                let output = format!("{}\n", args.join(" "));
                write_stdout(&output, redirect_operator, output_file);
            },
            
            //pwd command
            ["pwd"] => {
                if let Ok(path) = env::current_dir() {
                    let output = format!("{}\n", path.display());
                    write_stdout(&output, redirect_operator, output_file);
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
                    let output = format!("{} is a shell builtin\n", cmd);
                    write_stdout(&output, redirect_operator, output_file);
                    continue;
                }else if let Some(path) = is_executable(cmd) {
                    let output = format!("{cmd} is {path}\n");
                    write_stdout(&output, redirect_operator, output_file);
                } else {
                    let output = format!("{cmd}: not found\n");
                    write_stdout(&output, redirect_operator, output_file);
                }
            },

            ["type", args @ ..] => {
                let output = format!("{}: not found\n", args[0]);
                write_stdout(&output, redirect_operator, output_file);
            },
            [cmd, args @ ..] => {
                if let Some(path) = is_executable(cmd) {

                    let mut command = Command::new(path);

                    command.arg0(cmd).args(args);
                        if let Some(file_name) = output_file {
                            let append =
                            redirect_operator == Some(">>")
                            || redirect_operator == Some("1>>")
                            || redirect_operator == Some("2>>");

                        let file = if append {
                            fs::OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open(file_name)
                                .unwrap()
                        } else {
                            fs::File::create(file_name).unwrap()
                        };

                        if redirect_operator == Some("2>")
                            || redirect_operator == Some("2>>")
                        {
                            command.stderr(Stdio::from(file));
                        } else {
                            command.stdout(Stdio::from(file));
                        }
                    }
                    command.status().unwrap();
                } else {
                    println!("{}: command not found", cmd);
                }
            }
        }
    }
}
