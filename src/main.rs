#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    loop{
        print!("$ ");
        io::stdout().flush().unwrap();
        let mut command = String::new();
        io::stdin().read_line(&mut command).unwrap();
        
        match command.trim().split_whitespace().collect::<Vec<&str>>().as_slice(){
            [] => continue,
            ["exit"] => break,
            ["echo", args @ ..] => println!("{}", args.join(" ")),
            ["type", args @ ("exit"|"echo"|"type")] => println!("{} is a shell builtin", args),
            ["type", args @ ..] => println!("{}: not found", args[0]),
            _ => println!("{}: command not found", command.trim()),
        }
    }
}
