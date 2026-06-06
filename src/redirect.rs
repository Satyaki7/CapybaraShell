use std::io::{Write};
use std::fs;

//checks what type of operation is happening and writes the output to the appropriate place

pub fn write_stdout(output: &str, redirect_operator: Option<&str>, output_file: Option<&str>){

    match redirect_operator {
        Some(">") | Some("1>") => {
            if let Some(file) = output_file {
                fs::write(file, output).unwrap();
            }
        }
        Some("2>") => {
            if let Some(file) = output_file {
                fs::File::create(file).unwrap();
            }

            print!("{}", output);
        }
        Some(">>") | Some("1>>") => {
            if let Some(file) = output_file {
                fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(file)
                    .unwrap()
                    .write_all(output.as_bytes())
                    .unwrap();
            }
        }   
        Some("2>>") => {
            if let Some(file) = output_file {
                fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(file)
                    .unwrap();
            }
            print!("{}", output);
        }
        _ => {
            print!("{}", output);
        }
    }
}