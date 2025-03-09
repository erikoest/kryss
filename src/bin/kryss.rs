extern crate kryss;

use kryss::Dictionary;
use kryss::Board;
use kryss::{KryssApp, KryssKeywordExpander};

use cmdui::CmdUI;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut dname = "dict.json";
    let mut arg_count = 1;

    loop {
        if args.len() <= arg_count {
            break;
        }

        match args[arg_count].as_str() {
            "-d" | "--dictionary" => {
                dname = &args[arg_count + 1];
                arg_count += 2;
            },
            _ => break,
        }
    }

    let mut dict = Dictionary::from_file(dname);
    let board = Board::from_file(&args[arg_count], &mut dict);

    let kw_exp = KryssKeywordExpander::new(&board);
    let mut kryssapp = KryssApp::new(dict, board);

    CmdUI::new(&mut kryssapp, Some(&kw_exp)).read_commands();
}
