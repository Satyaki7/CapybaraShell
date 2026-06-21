use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};

use std::cell::RefCell;
use std::fs;
use std::io::{self, Write};

use crate::trie::Trie;
use std::process::Command;
use crate::command::COMPLETIONS;

pub struct ShellHelper {
    pub trie: Trie,
    pub last_tab: RefCell<Option<String>>,
}

impl Helper for ShellHelper {}

impl Hinter for ShellHelper {
    type Hint = String;
}

impl Highlighter for ShellHelper {}

impl Validator for ShellHelper {}

fn longest_common_prefix(strings: &[String]) -> String {
    if strings.is_empty() {
        return String::new();
    }

    let mut prefix = strings[0].clone();

    for s in &strings[1..] {
        while !s.starts_with(&prefix) {
            prefix.pop();

            if prefix.is_empty() {
                break;
            }
        }
    }

    prefix
}

impl Completer for ShellHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {

        let start = line[..pos]
            .rfind(' ')
            .map(|i| i + 1)
            .unwrap_or(0);

        // -------------------------------------------------
        // Registered completer scripts
        // -------------------------------------------------

        if line[..pos].ends_with(' ') {
            let command = line[..pos].trim();

            let completions = COMPLETIONS.lock().unwrap();

            if let Some(script_path) = completions.get(command) {
                if let Ok(output) = Command::new(script_path).output() {
                    let stdout = String::from_utf8_lossy(&output.stdout);

                    if let Some(candidate) = stdout.lines().next() {
                        return Ok((
                            pos,
                            vec![Pair {
                                display: candidate.to_string(),
                                replacement: format!("{} ", candidate),
                            }],
                        ));
                    }
                }
            }
        }

        // -------------------------------------------------
        // Filename completion
        // -------------------------------------------------

        if !line[..start].trim().is_empty() {
            let prefix = &line[start..pos];

            let (dir, file_prefix, replacement_prefix) =
                if let Some(idx) = prefix.rfind('/') {
                    (
                        &prefix[..idx],
                        &prefix[idx + 1..],
                        &prefix[..idx + 1],
                    )
                } else {
                    (".", prefix, "")
                };

            let mut matches: Vec<(String, bool)> = Vec::new();

            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let file_name = entry.file_name();
                    let file_name = file_name.to_string_lossy();
                    let is_dir = entry.path().is_dir();

                    if file_name.starts_with(file_prefix) {
                        matches.push((
                            format!("{}{}", replacement_prefix, file_name),
                            is_dir,
                        ));
                    }
                }
            }

            // No matches -> bell
            if matches.is_empty() {
                *self.last_tab.borrow_mut() = None;

                print!("\x07");
                io::stdout().flush().unwrap();

                return Ok((pos, Vec::new()));
            }

            // Single match -> complete fully
            if matches.len() == 1 {
                *self.last_tab.borrow_mut() = None;

                let (completed, is_dir) = &matches[0];

                return Ok((
                    start,
                    vec![Pair {
                        display: if *is_dir {
                            format!("{}/", completed)
                        } else {
                            completed.clone()
                        },
                        replacement: if *is_dir {
                            format!("{}/", completed)
                        } else {
                            format!("{} ", completed)
                        },
                    }],
                ));
            }

            // Multiple matches -> compute LCP
            matches.sort_by(|a, b| a.0.cmp(&b.0));

            let names: Vec<String> =
                matches.iter().map(|(name, _)| name.clone()).collect();

            let lcp = longest_common_prefix(&names);

            let typed = format!("{}{}", replacement_prefix, file_prefix);

            // If LCP extends what user typed, autocomplete immediately
            if lcp.len() > typed.len() {
                *self.last_tab.borrow_mut() = None;

                return Ok((
                    start,
                    vec![Pair {
                        display: lcp.clone(),
                        replacement: lcp,
                    }],
                ));
            }

            // Otherwise use bell / second-tab behaviour
            let mut last = self.last_tab.borrow_mut();

            if last.as_deref() == Some(line) {
    *last = None;

    let display_strs: Vec<String> = matches
        .iter()
        .map(|(name, is_dir)| {
            if *is_dir {
                format!("{}/", name)
            } else {
                name.clone()
            }
        })
        .collect();

    print!("\n");
    println!("{}", display_strs.join("  "));
    print!("$ {}", line);
    io::stdout().flush().unwrap();

    return Ok((pos, Vec::new()));
}

*last = Some(line.to_string());

print!("\x07");
io::stdout().flush().unwrap();

return Ok((pos, Vec::new()));
        }

        // -------------------------------------------------
        // Command completion
        // -------------------------------------------------

        let prefix = &line[start..pos];
        let mut matches = self.trie.get_matches(prefix);

        if matches.is_empty() {
            *self.last_tab.borrow_mut() = None;

            print!("\x07");
            io::stdout().flush().unwrap();

            return Ok((pos, Vec::new()));
        }

        if matches.len() == 1 {
            *self.last_tab.borrow_mut() = None;

            let completed = &matches[0];

            return Ok((
                start,
                vec![Pair {
                    display: completed.clone(),
                    replacement: format!("{} ", completed),
                }],
            ));
        }

        if let Some(common_prefix) = self.trie.autocomplete(prefix) {
            if common_prefix.len() > prefix.len() {
                *self.last_tab.borrow_mut() = None;

                return Ok((
                    start,
                    vec![Pair {
                        display: common_prefix.clone(),
                        replacement: common_prefix,
                    }],
                ));
            }
        }

        matches.sort();

        let mut last = self.last_tab.borrow_mut();

        if last.as_deref() == Some(line) {
            // Second TAB -> show matches
            *last = None;

            let display_strs: Vec<String> = matches.clone();

            print!("\n");
            println!("{}", display_strs.join("  "));
            print!("$ {}", line);
            io::stdout().flush().unwrap();

            Ok((pos, Vec::new()))
        } else {
            // First TAB -> bell only
            *last = Some(line.to_string());

            print!("\x07");
            io::stdout().flush().unwrap();

            Ok((pos, Vec::new()))
        }
    }
}