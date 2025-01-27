use crate::{Board, State};
use crate::Dictionary;

use rustyline::hint::Hinter;
use rustyline::Helper;
use rustyline::{CompletionType, Context, Editor, Config};
use rustyline::completion::{Completer, Pair};
use rustyline::validate::{Validator, ValidationResult, ValidationContext};
use rustyline::highlight::{Highlighter};
use rustyline::error::ReadlineError;
extern crate term_size;
use console::{Term, Key};

use std::collections::HashMap;
use std::collections::HashSet;
use std::io::stdin;
use std::ops::{Range, RangeFrom};
use std::fs;
use std::cmp::{min, max};
use std::io::Write;
use std::io;

const COMMANDS: &'static [&'static str] = &[
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
    "lookup <key> <length> [<hint>]",
    "set colors <bool>",
    "store board <filename>",
    "store dictionary <filename>",
    "add <key> <word>",
    "help",
];

struct CommandLine {
    line: String
}

impl CommandLine {
    fn new(line: String) -> Self {
        Self {
            line: line
        }
    }

    fn as_str(&self) -> &str {
        return &self.line;
    }

    fn parts(&self) -> CommandLineIterator {
        return CommandLineIterator::new(self);
    }
}

struct CommandPart<'a> {
    slice: &'a str,
    is_quoted: bool,
    is_error: bool,
}

impl<'a> CommandPart<'a> {
    fn new(slice: &'a str) -> Self {
        let is_quoted = slice.find(' ').is_some();
        Self {
            slice: slice,
            is_quoted: is_quoted,
            is_error: false,
        }
    }

    fn error(slice: &'a str) -> Self {
        Self {
            slice: slice,
            is_quoted: false,
            is_error: true,
        }
    }

    fn into_string(&self) -> Result<String, String> {
        if self.is_error {
            return Err(self.to_string());
        }
        else {
            return Ok(self.slice.to_string());
        }
    }

    fn starts_with(&self, other: &CommandPart) -> bool {
        return self.slice.starts_with(other.slice);
    }

    fn to_string(&self) -> String {
        if self.is_error {
            return format!("Bad_command: {}", self.slice);
        }
        else if self.is_quoted {
            return format!("'{}'", self.slice);
        }
        else {
            return self.slice.to_string();
        }
    }
}

impl<'a> PartialEq for CommandPart<'a> {
    fn eq(&self, other: &CommandPart) -> bool {
        return self.slice == other.slice;
    }
}

struct CommandLineIterator<'a> {
    line: &'a CommandLine,
    position: usize,
}

impl<'a> CommandLineIterator<'a> {
    fn new(line: &'a CommandLine) -> Self {
        Self {
            line: line,
            position: 0,
        }
    }

    fn slice(&self, r: Range<usize>) -> &'a str {
        return &self.line.as_str()[r];
    }

    fn slice_from(&self, r: RangeFrom<usize>) -> &'a str {
        return &self.line.as_str()[r];
    }

    fn find_from(&self, r: RangeFrom<usize>, c: char) -> Option<usize> {
        let start = r.start;
        if let Some(nextpos) = &self.slice_from(r).find(c) {
            return Some(start + nextpos);
        }
        else {
            return None;
        }
    }

    fn find_from_to(&self, r: Range<usize>, c: char) -> Option<usize> {
        let start = r.start;
        if let Some(nextpos) = &self.slice(r).find(c) {
            return Some(start + nextpos);
        }
        else {
            return None;
        }
    }

    fn len(&self) -> usize {
        return self.line.as_str().len();
    }

    fn char_is(&self, pos: usize, c: char) -> bool {
        return self.line.as_str()[pos..].starts_with(c);
    }

    fn next_or_error(&mut self) -> Result<String, String> {
        if let Some(p) = self.next() {
            return p.into_string();
        }
        else {
            return Err("Bad command".to_string());
        }
    }

    fn next_word(&mut self) -> Result<Option<String>, String> {
        if let Some(p) = self.next() {
            if p.slice.is_empty() {
                return Ok(None);
            }
            else {
                return Ok(Some(p.into_string()?));
            }
        }
        else {
            return Ok(None);
        }
    }
}

impl<'a> Iterator for CommandLineIterator<'a> {
    type Item = CommandPart<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let pos = self.position;

        if pos > self.len() {
            return None;
        }

        // If we are exactly at the end of line, a space was added after
        // the last part. Add an empty final part to signify this.
        if pos == self.len() {
            self.position += 1;
            return Some(CommandPart::new(""));
        }

        if self.char_is(pos, '\'') {
            // Quoted part
            if let Some(nextpos) = &self.find_from(pos + 1.., '\'') {
                // Located second quote
                if nextpos + 1 != self.len() {
                    // Not end of line
                    if self.char_is(nextpos + 1, ' ') {
                        // Space after quote
                        self.position = nextpos + 2;
                    }
                    else {
                        // No space found after quote. Treat as error
                        self.position = nextpos + 1;
                        return Some(CommandPart::error(
                            &self.slice(pos..*nextpos)));
                    }
                }
                else {
                    // End of line
                    self.position = nextpos + 2;
                }

                return Some(CommandPart::new(&self.slice(pos + 1..*nextpos)));
            }
            else {
                // No second quote found. Treat the rest of the string as part.
                self.position = self.len() + 1;
                return Some(CommandPart::new(&self.slice_from(pos + 1..)));
            }
        }
        else {
            // Unquoted part
            if let Some(nextpos) = &self.find_from(pos.., ' ') {
                // Not end of line
                // Check that part doesn't contain a quote
                self.position = nextpos + 1;
                if let Some(_) = self.find_from_to(pos..*nextpos, '\'') {
                    return Some(CommandPart::error(&self.slice(pos..*nextpos)));
                }
                else {
                    return Some(CommandPart::new(&self.slice(pos..*nextpos)));
                }
            }
            else {
                // End of line.
                self.position = self.len() + 1;
                // Check that part doesn't contain a quote
                if let Some(_) = self.find_from(pos.., '\'') {
                    return Some(CommandPart::error(&self.slice_from(pos..)));
                }
                else {
                    return Some(CommandPart::new(&self.slice_from(pos..)));
                }
            }
        }
    }
}

#[derive(Helper)]
struct CommandHelper {
    completer: CommandCompleter,
}

struct CommandCompleter {
    candidates: HashMap<String, Vec<String>>,
    keys: Vec<String>,
}

impl CommandCompleter {
    fn new(board: &Board) -> Self {
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

    fn expand_filename(&self, path: &str) -> Vec<String> {
        let mut ret = vec!();
        let (dpart, fpart);

        if let Some(pos) = path.rfind('/') {
            dpart = &path[0..pos + 1];
            fpart = &path[pos + 1..];
        }
        else if path == "." {
            dpart = "./";
            fpart = "";
        }
        else {
            dpart = "./";
            fpart = path;
        }

        if let Ok(dir) = fs::read_dir(dpart) {
            for entry_result in dir {
                let entry = entry_result.unwrap();
                if let Some(fstr) = entry.file_name().to_str() {
                    if fstr.starts_with(fpart) {
                        let mut fstring = fstr.to_string();
                        if entry.path().is_dir() {
                            fstring.push('/');
                        }

                        ret.push(format!("{}{}", dpart, fstring));
                    }
                }
            }
        }

        return ret;
    }

    fn expand_candidates(&self, key: &str) -> Vec<String> {
        if let Some(v) = self.candidates.get(key) {
            return v.clone();
        }
        else {
            return vec!();
        }
    }

    fn complete(&self, line: &str, _pos: usize, _ctx: &Context)
        -> rustyline::Result<(usize, Vec<Pair>)>
    {
        let mut pairs = HashMap::new();

        let line_cl = CommandLine::new(line.to_string());
        let lwords: Vec<CommandPart> = line_cl.parts().collect();

        // Return empty completion list if line has errors
        for w in &lwords {
            if w.is_error {
                return Ok((0, vec!()));
            }
        }

        // Loop over all commands
        'commands: for cmd in COMMANDS {
            let mut prefix = "".to_string();
            let mut previous_word = "".to_string();

            let cmd_cl = CommandLine::new(cmd.to_string());
            let cmd_vec: Vec<CommandPart> = cmd_cl.parts().collect();
            let cmd_vec_len = cmd_vec.len();

            // Loop over command parts
            'parts: for (i, cp) in cmd_vec.into_iter().enumerate() {
                if i == lwords.len() {
                    continue 'commands;
                }

                let lpart = &lwords[i];

                let keys = match cp.slice {
                    "<key>"       => self.keys.clone(),
                    "<filename>"  => self.expand_filename(lwords[i].slice),
                    "<bool>"      => vec!["on".to_string(), "off".to_string()],
                    "<candidate>" => self.expand_candidates(&previous_word),
                    s             => vec![s.to_string()],
                };

                let mut got_matches = false;

                for k in keys.iter().map(|k| CommandPart::new(&k)) {
                    if i == lwords.len() - 1 {
                        // Unfinished (last) part. Accept partial match.
                        if !k.starts_with(&lpart) {
                            continue;
                        }
                    }
                    else {
                        // Not last part. Require complete match for keywords
                        // No check for variable parameters
                        if !cp.slice.starts_with('<') {
                            if *lpart != k {
                                continue;
                            }
                        }

                        // The line matches the complete part. Add it to
                        // prefix, skip to next command part, continue
                        // matching.
                        prefix.push_str(&lpart.to_string());
                        prefix.push_str(" ");

                        previous_word = lpart.to_string();
                        continue 'parts;
                    }

                    // All line parts match the corresponding part in a
                    // command. Create a replacement pair.
                    let mut replacement = prefix.clone();
                    replacement.push_str(&k.to_string());

                    if cmd_vec_len > i + 1 {
                        replacement.push(' ');
                    }

                    let display = k.to_string();

                    pairs.insert(display.clone(), Pair {
                        display: display,
                        replacement: replacement,
                    });
                    got_matches = true;
                }

                if !got_matches {
                    continue 'commands;
                }
            }
        }

        let mut pairvec: Vec<Pair> = pairs.into_values().collect();
        pairvec.sort_by(|a, b| a.display.cmp(&b.display));

        Ok((0, pairvec))
    }
}

impl Completer for CommandHelper {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize, ctx: &Context)
                -> rustyline::Result<(usize, Vec<Pair>)>
    {
        self.completer.complete(line, pos, ctx)
    }
}

impl Hinter for CommandHelper {
    type Hint = String;

    fn hint(&self, _line: &str, _pos: usize, _ctx: &Context)
            -> Option<String>
    {
        None
    }
}

impl Validator for CommandHelper {
    fn validate(&self, _ctx: &mut ValidationContext)
                -> rustyline::Result<ValidationResult>
    {
        Ok(ValidationResult::Valid(None))
    }
}

impl Highlighter for CommandHelper {}

pub struct Cli {
    dict: Dictionary,
    board: Board,
}

impl Cli {
    pub fn new(dict: Dictionary, board: Board) -> Self
    {
        Self {
            dict: dict,
            board: board,
        }
    }

    pub fn read_commands(&mut self) {
        self.solve();
        self.show_board();

        let config = Config::builder()
            .completion_type(CompletionType::List)
            .build();

        let mut editor = Editor::with_config(config).unwrap();

        loop {
            let helper = CommandHelper {
                completer: CommandCompleter::new(&self.board),
            };

            editor.set_helper(Some(helper));
            let readline = editor.readline("> ");
            match readline {
                Ok(line) => {
                    let _ = editor.add_history_entry(&line);
                    let cl = CommandLine::new(line);
                    if let Err(e) = self.parse_and_execute(&cl) {
                        println!("{}", e);
                    }
                },
                Err(ReadlineError::Interrupted) => {
                    continue;
                }
                Err(ReadlineError::Eof) => {
                    break;
                }
                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                }
            }
        }

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

    fn confirm_yes_no(&self) -> bool {
        let mut buf = String::new();
        stdin().read_line(&mut buf).unwrap();
        return buf.trim().to_lowercase() == "y" || buf.trim().len() == 0;
    }

    fn parse_int(&self, intstr: &str) -> Result<usize, String> {
        if let Ok(length) = intstr.parse() {
            return Ok(length);
        }
        else {
            return Err(format!("Expected integer, got '{}'", intstr));
        }
    }

    fn parse_bool(&self, boolstr: &str) -> Result<bool, String> {
        return match boolstr {
            "on" | "true" | "1" => {
                Ok(true)
            },
            "off" | "false" | "0" => {
                Ok(false)
            },
            _ => {
                Err(format!("Expected boolean, got '{}'", boolstr))
            },
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

    fn parse_and_execute(&mut self, cl: &CommandLine) -> Result<(), String> {
        let mut parts = cl.parts();

        match parts.next_or_error()?.as_str() {
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
                let key_part = parts.next_or_error()?;
                let key = self.find_word(&key_part)?;

                self.show_crossing(key);
            },
            "candidates" => {
                let key_part = parts.next_or_error()?;
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
                let key_part = parts.next_or_error()?;
                let key = self.find_word(&key_part)?;

                self.info_word(key);
            },
            "set" => {
                match parts.next_or_error()?.as_str() {
                    "colors" => {
                        self.set_colors(
                            self.parse_bool(&parts.next_or_error()?)?);
                    },
                    _ => {
                        return Err("Bad command".to_string());
                    },
                }
            }
            "place" => {
                let key_part = parts.next_or_error()?;
                let key = self.find_word(&key_part)?;
                let word = parts.next_or_error()?;

                self.place(key, &word);
            },
            "lookup" => {
                let word = parts.next_or_error()?;
                let param = &parts.next_or_error()?;
                if let Ok(length) = self.parse_int(param) {
                    self.lookup(&word, length, None);
                }
                else {
                    self.lookup(&word, param.len(), Some(param));
                }
            },
            "store" => {
                match parts.next_or_error()?.as_str() {
                    "board" => {
                        self.store_board(parts.next_word()?.as_deref());
                    },
                    "dictionary" => {
                        self.store_dictionary(parts.next_word()?.as_deref());
                    },
                    _ => {
                        return Err("Bad command".to_string());
                    },
                }
            },
            "add " => {
                self.add_word(&parts.next_or_error()?,
                              &parts.next_or_error()?);
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

    fn wait_for_key(&self) -> Key {
        let term = Term::stdout();
        return term.read_key().unwrap();
    }

    fn print_columns(&self, lines: &[String], max_line: usize) {
        let (term_w, term_h) = term_size::dimensions().unwrap_or((80, 25));
        let min_padding = 2;
        let cols = term_w/(max_line + min_padding);
        let cwidth = term_w/cols;
        let end_row = (lines.len() + cols - 1) / cols;
        let page_size = term_h - 1;
        let is_paged = end_row > page_size;
        let mut position = 0;

        'outer: loop {
            let lstart = position*cols;
            let lend = min((position + page_size)*cols, lines.len());
            let mut i = 0;

            if is_paged {
                print!("\r");
            }

            for l in &lines[lstart..lend] {
                i = (i + 1)%cols;

                if i == 0 {
                    println!("{}", l);
                }
                else {
                    print!("{: <1$}", l, cwidth);
                }
            }

            if i != 0 {
                println!();
            }

            if !is_paged {
                break;
            }

            print!("--More--");
            io::stdout().flush().unwrap();

            loop {
                match self.wait_for_key() {
                    Key::Home => {
                        if position > 0 {
                            position = 0;
                            break;
                        }
                    },
                    Key::End => {
                        if position + page_size < end_row {
                            position = end_row - page_size;
                            break;
                        }
                    },
                    Key::PageUp | Key::Char('b') => {
                        if position > page_size {
                            position -= page_size;
                            break;
                        }
                        else if position > 0 {
                            position = 0;
                            break;
                        }
                    },
                    Key::Char(' ') | Key::PageDown => {
                        if position + page_size*2 < end_row {
                            position += page_size;
                            break;
                        }
                        else if position + page_size < end_row {
                            position = end_row - page_size;
                            break;
                        }
                    },
                    Key::ArrowUp => {
                        if position > 0 {
                            position -= 1;
                            break;
                        }
                    },
                    Key::Enter | Key::ArrowDown => {
                        if position + page_size < end_row {
                            position += 1;
                            break;
                        }
                    },
                    Key::Char('q') | Key::CtrlC | Key::Escape => {
                        break 'outer;
                    },
                    _ => { },
                }
            }
        }

        if is_paged {
            // Remove --More-- prompt
            print!("\r");
            print!("        ");
            print!("\r");
            io::stdout().flush().unwrap();
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
        println!(r#"solve
words
placed
unpaced
missing
ambiguous
crossing <key>
candidates <key>
solution
board
info <key>
place <key> <word>
lookup <key> <length> [<hint>]
store board <filename>
store dictionary <filename>
set colors on/off
add <key> <word>
help"#);
    }
}
