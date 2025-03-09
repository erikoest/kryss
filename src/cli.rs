use crate::{Board, State};
use crate::Dictionary;

extern crate term_size;
use cmdui::{CmdApp, KeywordExpander, CommandPart};
use std::collections::HashMap;
use std::collections::HashSet;
use std::io::stdin;
use std::cmp::max;

const COMMAND_LIST: &'static [&'static str] = &[
    "solve",
    "words",
    "placed",
    "unplaced",
    "missing",
    "ambiguous",
    "crossing <key>",
    "candidates <key>",
    "solution",
    "board",
    "info <key>",
    "place <key> <candidate>",
    "lookup <key> [<length>|<hint>]",
    "set colors <bool>",
    "store board <filename>",
    "store dictionary <filename>",
    "add <key> <word>",
    "help",
];

pub struct KryssKeywordExpander {
    keys: Vec<String>,
    candidates: HashMap<String, Vec<String>>,
}

impl KryssKeywordExpander {
    pub fn new(board: &Board) -> Self {
        let keys = board.words.iter()
            .filter(|w| w.key.is_some())
            .map(|w| w.key.as_ref().unwrap().clone())
            .collect();

        let mut candidates: HashMap<String, Vec<String>> = HashMap::new();

        for w in &board.words {
            if let Some(k) = &w.key {
                if let Some(c) = candidates.remove(k) {
                    let mut hs: HashSet<&String> =
                        HashSet::from_iter(c.iter().collect::<Vec<&String>>());
                    for cw in &w.candidates {
                        hs.insert(&cw);
                    }
                    candidates.insert(k.clone(), hs.drain().map(
                        |cw| cw.to_string()).collect());
                }
                else {
                    candidates.insert(k.clone(), w.candidates.clone());
                }
            }
        }

        for i in 0..board.words.len() {
            let w = &board.words[i];
            candidates.insert(i.to_string(), w.candidates.clone());
        }

        Self {
            keys: keys,
            candidates: candidates,
        }
    }

    fn expand_candidates(&self, key: &str) -> Vec<String> {
        if let Some(v) = self.candidates.get(key) {
            return v.clone();
        }
        else {
            return vec!();
        }
    }

    fn expand_keys(&self, _: &str) -> Vec<String> {
        return self.keys.clone();
    }
}

impl KeywordExpander for KryssKeywordExpander {
    fn command_list<'a>(&self) -> &'a [&'a str] {
        return COMMAND_LIST;
    }

    fn expand_keyword(&self, cp: &CommandPart, parts: &Vec<String>)
                      -> Vec<String> {
        let lpart = &parts[parts.len() - 1];

        match cp.as_str() {
            "<filename>"  => { self.expand_filename(lpart) },
            "<candidate>" => { self.expand_candidates(
                &parts[parts.len() - 2]) },
            "<key>"       => { self.expand_keys(lpart) },
            "<bool>"      => { vec!["on".to_string(), "off".to_string()] },
            s             => { vec![s.to_string()] },
        }
    }
}

pub struct KryssApp {
    dict: Dictionary,
    board: Board,
}

impl KryssApp {
    pub fn new(dict: Dictionary, board: Board) -> Self
    {
        Self {
            dict: dict,
            board: board,
        }
    }

    fn find_word(&self, key: &str) -> Result<usize, String> {
        if let Ok(i) = key.parse::<usize>() {
            return Ok(i);
        }

        let mut hits = vec!();

        for (a, w) in self.board.words.iter().enumerate() {
            if let Some(wkey) = &w.key {
                if wkey == key {
                    hits.push(a);
                    continue;
                }
            }

            if w.placed {
                if w.candidates[0] == key {
                    hits.push(a);
                    continue;
                }
            }
        }

        match hits.len() {
            0 => {
                return Err(format!("Word not found"));
            },
            1 => {
                return Ok(hits[0]);
            },
            _ => {
                for a in &hits {
                    println!("{}", self.board.format_word(*a))
                }

                let mut buf = String::new();
                stdin().read_line(&mut buf).unwrap();
                if let Ok(i) = buf.trim().parse::<usize>() {
                    if hits.contains(&i) {
                        return Ok(i);
                    }
                    else {
                        return Err(format!("Invalid word {}", i));
                    }
                }
                else {
                    return Err(format!("Invalid input: {}", buf));
                }
            },
        }
    }

    fn solve(&mut self) {
        self.board.solve_repeated(&mut self.dict);

        if self.board.state == State::Solved {
            println!("Solved");
            println!();
            self.show_board();
        }
    }

    fn show_words(&self, skip_placed: bool, skip_missing: bool,
                  skip_ambiguous: bool) {
        let mut width = 0;
        let mut lines = vec!();

        for (a, w) in self.board.words.iter().enumerate() {
            if w.placed && skip_placed {
                continue;
            }

            if w.is_missing() && skip_missing {
                continue;
            }

            if w.is_ambiguous() && skip_ambiguous {
                continue;
            }

            let line = format!("{}", self.board.format_word(a));
            width = max(width, line.len());
            lines.push(line);
        }

        self.print_columns(&lines, width);
    }

    fn show_solution(&self) {
        println!("{}", self.board.words.iter().enumerate()
                 .filter(|(_, w)|
                         w.key.is_none()
                 )
                 .map(
                     |(i, _)|
                     self.board.get_hints(i)
                 )
                 .collect::<Vec<String>>()
                 .join(" "));
    }

    fn show_board(&self) {
        println!("{}", self.board.to_string());
        println!();
    }

    fn show_crossing(&self, key: usize) {
        if !self.board.crossings.contains_key(&key) {
            println!("No crossing words for key");
            return;
        }

        println!("{}", self.board.format_word(key));
        self.board.show_crossing(key);
    }

    fn show_candidates(&self, key: usize) {
        for c in &self.board.words[key].candidates {
            println!("  {}", c);
        }
    }

    fn info_word(&self, key: usize) {
        println!("{}", self.board.format_word(key));
        self.board.info_word(&key);
        println!();
        self.board.show_crossing(key);
    }

    fn set_colors(&mut self, on: bool) {
        self.board.colors = on;
    }

    fn place(&mut self, key: usize, word: &str) {
        if self.board.words[key].length != word.chars().count() {
            println!("Invalid length.");
            return;
        }

        self.board.place(key, Some(word.to_string()), &mut self.dict);

        // Add word to dictionary if missing
        if let Some(k) = &self.board.words[key].key {
            self.dict.add_word(&k, word);
        }
    }

    fn lookup(&mut self, key: &str, length: usize, opt_hint: Option<&str>) {
        for w in &self.dict.lookup(key, length, opt_hint) {
            print!("{} ", w);
        }
        println!();
    }

    fn store_board(&mut self, opt_fname: Option<&str>) {
        self.board.write_to_file(opt_fname);
    }

    fn store_dictionary(&mut self, opt_fname: Option<&str>) {
        self.dict.write_to_file(opt_fname);
    }

    fn add_word(&mut self, key: &str, word: &str) {
        self.dict.add_word(key, word);
    }

    fn help(&self) {
        println!("{}", COMMAND_LIST.into_iter()
                 .map(|c| c.replace("<bool>", "on/off"))
                 .collect::<Vec<String>>()
                 .join("\n")
        );
    }
}

impl CmdApp for KryssApp {
    fn command_list<'a>(&self) -> &'a [&'a str] {
        return COMMAND_LIST;
    }

    fn execute_line(&mut self, cmd: &str, args: &Vec<String>)
                    -> Result<(), String> {
        match cmd {
            "solve" => {
                self.solve();
            },
            "words" => {
                self.show_words(false, false, false);
            },
            "placed" => {
                self.show_words(false, true, true);
            },
            "unplaced" => {
                self.show_words(true, false, false);
            },
            "missing" => {
                self.show_words(true, false, true);
            },
            "ambiguous" => {
                self.show_words(true, true, false);
            },
            "crossing" => {
                <dyn CmdApp>::expects_num_arguments(args, 1)?;
                let key_part = &args[0];
                let key = self.find_word(&key_part)?;

                self.show_crossing(key);
            },
            "candidates" => {
                <dyn CmdApp>::expects_num_arguments(args, 1)?;
                let key_part = &args[0];
                let key = self.find_word(&key_part)?;

                self.show_candidates(key);
            },
            "solution" => {
                self.show_solution();
            },
            "board" => {
                self.show_board();
            },
            "info" => {
                <dyn CmdApp>::expects_num_arguments(args, 1)?;
                let key_part = &args[0];
                let key = self.find_word(&key_part)?;

                self.info_word(key);
            },
            "set colors" => {
                <dyn CmdApp>::expects_num_arguments(args, 1)?;
                self.set_colors(
                    <dyn CmdApp>::parse_bool(&args[0])?);
            },
            "place" => {
                <dyn CmdApp>::expects_num_arguments(args, 2)?;
                let key_part = &args[0];
                let key = self.find_word(&key_part)?;
                let word = &args[1];

                self.place(key, &word);
            },
            "lookup" => {
                <dyn CmdApp>::expects_num_arguments(args, 2)?;
                let word = &args[0];
                let param = &args[1];
                if let Ok(length) = <dyn CmdApp>::parse_int(param) {
                    self.lookup(&word, length, None);
                }
                else {
                    self.lookup(&word, param.len(), Some(param));
                }
            },
            "store board" => {
                self.store_board(<dyn CmdApp>::opt_part(args, 0));
            },
            "store dictionary" => {
                self.store_dictionary(<dyn CmdApp>::opt_part(args, 0));
            },
            "add " => {
                <dyn CmdApp>::expects_num_arguments(args, 2)?;
                self.add_word(&args[0], &args[1]);
            },
            "help" => {
                self.help();
            }
            "" => { },
            _ => {
                return Err("Bad command".to_string());
            }
        }

        Ok(())
    }

    fn startup(&mut self) {
        self.solve();
        self.show_board();
    }

    fn exit(&mut self) {
        if self.board.changed {
            println!("Save changes to {}? (Y/n)", self.board.filename);
            if self.confirm_yes_no() {
                self.store_board(None);
            }
        }

        if self.dict.changed {
            println!("Save dictionary to {}? (Y/n)", self.dict.filename);
            if self.confirm_yes_no() {
                self.store_dictionary(None);
            }
        }
    }
}
