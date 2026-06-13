use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};
use std::fs;


use std::io::{self, Write};
use crate::trie::Trie;

pub struct ShellHelper {
    pub trie: Trie,
}

impl Helper for ShellHelper {}

impl Hinter for ShellHelper {
    type Hint = String;
}

impl Highlighter for ShellHelper {}

impl Validator for ShellHelper {}

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

        if !line[..start].trim().is_empty() {

    let prefix = &line[start..pos];
// The user might be typing:
//   hello
// or
//   path/to/hel
//
// Split it into:
//   dir    = "." or "path/to"
//   prefix = "hello" or "hel"

    let (dir, file_prefix, replacement_prefix) = if let Some(idx) = prefix.rfind('/') {
        (
            &prefix[..idx],            // directory to search
            &prefix[idx + 1..],        // filename prefix
            &prefix[..idx + 1],        // "path/to/"
        )
    } else {
        (
            ".",
            prefix,
            "",
        )
    };

    // storing the matching directories 
    let mut matches = Vec::new();

    // Search the appropriate directory.
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();
            let is_dir = entry.path().is_dir();

            // Match only against the filename part.
            if file_name.starts_with(file_prefix) {
                // Reconstruct the full completion.
                matches.push((format!("{}{}", replacement_prefix, file_name), is_dir));
            }
        }
    }
    // No matching files
    if matches.is_empty() {
        return Ok((pos, Vec::new()));
    }

    // Exactly one match: complete it and add a trailing space
    if matches.len() == 1 {
        let (completed, is_dir) = &matches[0];
        return Ok((
            start,
            vec![Pair {
                display: completed.clone(),
                replacement: if *is_dir {
                    format!("{}/", completed)
                } else {
                    format!("{} ", completed)
                },
            }],
        ));
    }

    // Multiple matches: let rustyline display them
    let pairs = matches
        .into_iter()
        .map(|m| Pair {
            display: m.clone(),
            replacement: m,
        })
        .collect();

    return Ok((start, pairs));
}

        let prefix = &line[start..pos];
        let matches = self.trie.get_matches(prefix);

        if matches.is_empty() {
            print!("\x07");
            io::stdout().flush().unwrap();
            return Ok((pos, Vec::new()));
        }

        if matches.len() == 1 {
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
                return Ok((
                    start,
                    vec![Pair {
                        display: common_prefix.clone(),
                        replacement: common_prefix,
                    }],
                ));
            }
        }

        print!("\x07");
        io::stdout().flush().unwrap();

        let pairs = matches.into_iter().map(|m| Pair {
            display: m.clone(),
            replacement: m,
        }).collect();

        Ok((start, pairs))
    }
}