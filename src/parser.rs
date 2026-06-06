// Vec<String> is a vector of strings [growing array]
//parses the command and returns a vector of strings

pub fn parse_command(command: &str) -> Vec<String>{
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