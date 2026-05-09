#[allow(unused_imports)]
use std::io::{self, Write};
use std::env;
use std::path::Path;
use std::fs;
use std::os::unix::fs::PermissionsExt;

fn is_builtin(cmd: &str) -> bool {
    matches!(cmd, "exit" | "echo" | "type")
}


fn main() {
    loop{
        print!("$ ");
        io::stdout().flush().unwrap();
        let mut command = String::new();
        io::stdin().read_line(&mut command).unwrap();
        
        match command
        .trim()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .as_slice(){

            [] => continue,
            ["exit"] => break,
            ["echo", args @ ..] => println!("{}", args.join(" ")),
            ["type", cmd] => {
                if is_builtin(cmd) {
                    println!("{} is a shell builtin ", cmd);
                    continue;
                }
                let path_var = env::var("PATH").unwrap_or_default();
                let mut found = false;

                //splitting PATH variable and checking each directory.
                 for dir in path_var.split(':') {
                    let full_path = Path::new(dir).join(cmd);

                    // check file exists
                    if full_path.exists() {

                        // check executable permission by reading the metadata and checking the permissions.
                        if let Ok(metadata) = fs::metadata(&full_path) {
                            let perms = metadata.permissions();

                            if perms.mode() & 0o111 != 0 {
                                println!("{} is {}", cmd, full_path.display());
                                found = true;
                                break;
                            }
                        }
                    }
                }
                if !found {
                    println!("{cmd}: not found");
                }
            },

            ["type", args @ ..] => println!("{}: not found", args[0]),
            _ => println!("{}: command not found", command.trim()),
        }
    }
}
