use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};

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
             return Ok((pos, Vec::new()));
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