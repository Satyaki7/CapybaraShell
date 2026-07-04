mod command;
mod completion;
mod executable;
mod parser;
mod redirect;
mod trie;

use command::BUILTINS;
use completion::ShellHelper;
use executable::get_all_executables;
use rustyline::history::DefaultHistory;
use rustyline::{CompletionType, Config, EditMode, Editor};
use std::cell::RefCell;
use trie::Trie;

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

    let mut rl = Editor::<ShellHelper, DefaultHistory>::with_config(config).unwrap();
    rl.set_helper(Some(helper));

    loop {
        match rl.readline("$ ") {
            Ok(line) => {
                let _ = rl.add_history_entry(line.as_str());
                if !command::execute(line) {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}
