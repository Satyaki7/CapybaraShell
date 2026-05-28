use std::io::{self, Write};
use std::env;
use std::path::Path;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::process::Command;

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

fn main() {
    loop{
        print!("$ ");
        io::stdout().flush().unwrap();
        let mut command = String::new();
        io::stdin().read_line(&mut command).unwrap();
        

        //split the command into parts and match on the first part to determine the action

        match command
        .trim()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .as_slice(){

            [] => continue,
            ["exit"] => break,
            ["echo", args @ ..] => println!("{}", args.join(" ")),
            
            //pwd command
            ["pwd"] => {
                if let Ok(path) = env::current_dir() {
                    println!("{}", path.display());
                } else {
                    println!("Error getting current directory");
                }
            },

            //cd command
            ["cd",dir] => {
                //using a //_ tells the compiler we are leaving the var intentinally unused.
                if let Err(_e) = env::set_current_dir(dir){
                    println!("{}: No such file or directory",dir);
                }
            }
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
                    let _result = Command::new(path)
                        .arg0(cmd)
                        .args(args)
                        .status() //lets the executable print directly in the console
                        .expect("{cmd}: command execution failed");
                } else {
                    println!("{}: command not found", cmd);
                }
            }
        }
    }
}
