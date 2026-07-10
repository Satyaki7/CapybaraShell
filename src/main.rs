mod command;
mod completion;
mod executable;
mod parser;
mod redirect;
mod trie;
mod builtins;

use command::BUILTINS;
use completion::ShellHelper;
use executable::get_all_executables;
use rustyline::history::DefaultHistory;
use rustyline::{CompletionType, Config, EditMode, Editor};
use std::cell::RefCell;
use trie::Trie;

use builtins::reap_jobs;

fn main() {
    let mut trie = Trie::new();
    for cmd in BUILTINS.keys() {
        trie.insert(cmd);
    }

    for cmd in get_all_executables() {
        trie.insert(&cmd);
    }

    let helper = ShellHelper {
        trie,
        last_tab: RefCell::new(None),
    };

    let config = Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .edit_mode(EditMode::Emacs)
        .build();

    let mut input = Editor::<ShellHelper, DefaultHistory>::with_config(config).unwrap();
    input.set_helper(Some(helper));

    loop {
        match input.readline("$ ") {
            Ok(line) => {
                let _ = input.add_history_entry(line.as_str());
                if !command::execute(line) {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}
