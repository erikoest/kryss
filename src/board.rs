use crate::word::Word;
use crate::dictionary::Dictionary;

use std::collections::HashMap;
use std::fs::read_to_string;
use std::fs::File;
use std::io::Write;
use colored::Colorize;
use std::cmp::{min, max};

#[derive(PartialEq)]
pub enum State {
    Unsolved,
    Unsolvable,
    Ambiguous,
    Solved,
}

pub struct Board {
    pub words: Vec<Word>,
    // a -> Vec<(b, ai, bi)>
    // a: index to word
    // b: index to crossing word
    // ai: index to crossing character in a
    // bi: index to crossing character in b
    pub crossings: HashMap<usize, Vec<(usize, usize, usize)>>,
    width: usize,
    height: usize,
    pub changed: bool,
    pub state: State,
    pub filename: String,
    pub colors: bool
}

impl Board {
    pub fn from_file(fname: &str, dict: &mut Dictionary) -> Self {
        let mut words = vec!();
        let mut width = 0;
        let mut height = 0;
        let mut prevlines = "".to_string();

        for line in read_to_string(fname).unwrap().lines() {
            if line.starts_with('#') {
                continue;
            }

            let trimmed = line.trim();

            if trimmed.is_empty() {
                continue;
            }

            if trimmed.ends_with(',') {
                prevlines.push_str(trimmed);
                continue;
            }

            let mut cat = prevlines.clone();
            cat.push_str(trimmed);
            prevlines = "".to_string();

            let parts: Vec<&str> = cat.split(',').collect();

            if parts[0] == "S" {
                // Solution line. The line specifies a list of words. Each
                // word has four fields (no key, no candidates). So the number
                // of parts is 1 + n*4
                assert!((parts.len() - 1)%4 == 0);

                for i in 0..(parts.len() - 1)/4 {
                    let w = Word::from_parts(&parts[i*4 + 1..i*4 + 5]);
                    words.push(w);
                }

                continue;
            }

            let word = Word::from_parts(&parts);

            width = max(width, word.xmax() + 1);
            height = max(height, word.ymax() + 1);
            words.push(word);
        }

        // Find crossing words
        let mut crossings = HashMap::new();
        for a in 0..words.len() {
            let mut a_crossings = vec!();
            let word_a = &words[a];

            for b in 0..words.len() {
                let word_b = &words[b];
                if a == b {
                    continue;
                }

                if word_a.is_conflicting(word_b) {
                    panic!("Words {} and {} are conflicting",
                           &word_a.to_string(), &word_b.to_string());
                }

                if word_a.is_crossing(word_b) {
                    let xi = if word_a.x > word_b.x {
                        word_a.x - word_b.x
                    }
                    else {
                        word_b.x - word_a.x
                    };

                    let yi = if word_a.y > word_b.y {
                        word_a.y - word_b.y
                    }
                    else {
                        word_b.y - word_a.y
                    };

                    if word_a.o.is_horizontal() {
                        a_crossings.push((b, xi, yi));
                    }
                    else {
                        a_crossings.push((b, yi, xi));
                    }
                }
            }

            crossings.insert(a, a_crossings);
        }

        let mut ret = Self {
            words: words,
            crossings: crossings,
            state: State::Unsolved,
            width: width,
            height: height,
            changed: false,
            filename: fname.to_string(),
            colors: true,
        };

        ret.refresh_candidates(dict);
        return ret;
    }

    pub fn write_to_file(&mut self, opt_fname: Option<&str>) {
        let mut filename = self.filename.clone();

        if let Some(fname) = opt_fname {
            filename = fname.to_string();
        }

        let mut file = File::create(&filename)
            .expect("Cannot create file");

        // Write words
        for w in &self.words {
            if w.key.is_some() {
                writeln!(file, "{}", w.to_string()).unwrap();
            }
        }

        // Write solution
        let mut started_solution = false;
        for sw in &self.words {
            if !sw.is_solution() {
                continue;
            }

            if !started_solution {
                write!(file, "S").unwrap();
                started_solution = true;
            }

            write!(file, ",{}", sw.to_string()).unwrap();
        }

        writeln!(file).unwrap();

        self.filename = filename;
        self.changed = false;
    }

    pub fn refresh_candidates(&mut self, dict: &mut Dictionary) {
        for i in 0..self.words.len() {
            let hint = &self.get_hints(i);
            let w = &mut self.words[i];

            if w.placed {
                continue;
            }

            if let Some(k) = &w.key {
                w.candidates = dict.lookup(&k, w.length, Some(&hint));
            }
        }
    }

    pub fn place(&mut self, ix: usize, opt_word: Option<String>,
                 dict: &mut Dictionary) {
        self.words[ix].place(opt_word);

        let w = self.words[ix].clone();

        let mut unplace = vec!();

        // Remove candidates from crossing words. Unplace placed words
        // if they conflict.
        for (b, ai, bi) in &self.crossings[&ix] {
            let xw = &mut self.words[*b];

            if xw.placed {
                if xw.char_at(*bi) != w.char_at(*ai) {
                    println!("Unplacing word {}", self.format_word(*b));
                    unplace.push(b.clone());
                }
            }
            else {
                let mut j = 0;

                while j < xw.candidates.len() {
                    if xw.candidates[j].chars().nth(*bi).unwrap() !=
                        w.char_at(*ai) {
                            xw.candidates.swap_remove(j);
                            continue;
                        }

                    j += 1;
                }
            }
        }

        for u in unplace {
            self.unplace(u, dict);
        }

        self.changed = true;
    }

    pub fn unplace(&mut self, ix: usize, dict: &mut Dictionary) {
        self.words[ix].unplace();

        self.refresh_candidates(dict);
    }

    // Check each word. Place it if a single candidate is found. Repeat until
    // no more candidates can be placed.
    pub fn solve_repeated(&mut self, dict: &mut Dictionary) {
        let mut done = false;

        while !done {
            done = true;

            for i in 0..self.words.len() {
                let w = &self.words[i];
                if w.placed {
                    continue;
                }

                if w.has_one_candidate() {
                    self.place(i, None, dict);
                    done = false;
                    println!("Placing word {}", self.format_word(i));
                }
            }
        }

        let mut max_candidates: i32 = -1;

        for w in &self.words {
            if w.placed {
                continue;
            }

            max_candidates = max(max_candidates,
                                 w.candidates.len().try_into().unwrap());
        }

        self.state = match max_candidates {
            -1 => State::Solved,
             0 => State::Unsolvable,
             1 => State::Unsolved,
             _ => State::Ambiguous,
        };
    }

    fn highlight(&self, c: char) -> String {
        if self.colors {
            return c.to_string().blue().to_string();
        }
        else {
            return c.to_string().bold().to_string();
        }
    }

    pub fn get_hints(&self, a: usize) -> String {
        let w = &self.words[a];
        let mut v = vec!['.'; w.length];

        if w.placed {
            // Word is placed. Just return the placed word
            return w.candidates[0].clone();
        }

        if self.crossings.contains_key(&a) {
            for (b, ai, bi) in &self.crossings[&a] {
                let wb = &self.words[*b];

                if wb.placed {
                    v[*ai] = wb.char_at(*bi);
                }
            }
        }

        return String::from_iter(v);
    }

    pub fn show_crossing(&self, a: usize) {
        let w = &self.words[a];

        let mut cross_formatted = vec!();
        let (mut xmin, mut xmax) = (w.xmin(), w.xmax());
        let (mut ymin, mut ymax) = (w.ymin(), w.ymax());

        let mut xind = vec!();
        let mut yind = vec!();

        let mut idx = vec!();
        for (b, ai, _) in  &self.crossings[&a] {
            idx.push((b, ai));
        }
        idx.sort_by(|(_, ai1), (_, ai2)| ai1.cmp(ai2));

        // Find dimension of the part to print
        for (b, _) in &idx {
            let wb = &self.words[**b];

            xmin = min(xmin, wb.xmin());
            xmax = max(xmax, wb.xmax());
            ymin = min(ymin, wb.ymin());
            ymax = max(ymax, wb.ymax());

            xind.push(wb.xmin());
            yind.push(wb.ymin());
        }

        let width = xmax - xmin + 2;
        let height = ymax - ymin + 1;
        let mut v = vec![" ".to_string(); width*height];

        for i in 1..height {
            v[i*width - 1] = "\n".to_string();
        }

        // First, print the main word
        for i in 0..w.length {
            let (x, y) = w.position_at_index(i);
            let ix = (y - ymin)*width + x - xmin;

            if w.placed {
                v[ix] = self.highlight(w.char_at(i));
            }
            else {
                v[ix] = self.highlight('.');
            }
        }

        // Then print each crossing word
        for (b, _) in &idx {
            let wb = &self.words[**b];

            cross_formatted.push(self.format_word(**b));

            // Get hints from crossing words.
            let hints;
            if wb.placed {
                hints = wb.candidates[0].clone();
            }
            else {
                hints = self.get_hints(**b);
            }

            for (ci, c) in hints.chars().into_iter().enumerate() {
                let (x, y) = wb.position_at_index(ci);
                let ix = (y - ymin)*width + x - xmin;

                if !w.position_in_word(x, y) {
                    v[ix] = c.to_string();
                }
                else if c != '.' {
                    v[ix] = self.highlight(c);
                }
            }
        }

        println!("{}", String::from_iter(v));
        println!();

        for c in cross_formatted {
            println!("{c}");
        }
    }

    pub fn info_word(&self, a: &usize) {
        let w = &self.words[*a];

        println!("Orientation: {}, X: {}, Y: {}, Length: {}",
                 w.o, w.x, w.y, w.length);
        if let Some(k) = &w.key {
            println!("Key: {}", k);
        }
        if w.placed {
            println!("Placed: {}", w.candidates[0]);
        }
        else if w.candidates.is_empty() {
            println!("No candidates");
        }
        else {
            println!("Candidates:");
            for c in &w.candidates {
                println!("  {}", c);
            }
        }
    }

    pub fn format_word(&self, i: usize) -> String {
        let w = &self.words[i];
        let ret;

        if let Some(k) = &w.key {
            if w.placed {
                ret = format!("[{}] {} = {}", i, k, w.candidates[0]);
            }
            else {
                ret = format!("[{}] {} = {} ?", i, k, self.get_hints(i));
            }
        }
        else {
            if w.placed {
                ret = format!("[{}] {}", i, w.candidates[0]);
            }
            else {
                ret = format!("[{}] {} ?", i, self.get_hints(i));
            }
        }

        if w.is_missing() {
            if self.colors {
                return ret.red().to_string();
            }
            else {
                return ret.bold().to_string();
            }
        }
        else if w.is_ambiguous() {
            // Italic
            if self.colors {
                return ret.purple().to_string();
            }
            else {
                return ret.italic().to_string();
            }
        }
        else {
            if self.colors {
                return ret.blue().to_string();
            }
            else {
                return ret;
            }
        }
    }
}

impl ToString for Board {
    fn to_string(&self) -> String {
        let width = self.width + 1;
        let height = self.height;
        let mut v = vec![" ".to_string(); width*height];

        for i in 1..height {
            v[i*width - 1] = "\n".to_string();
        }

        // First draw unplaced words with dots only
        for w in &self.words {
            if w.placed {
                continue;
            }

            for (x, y, c) in w {
                v[y*width + x] = c.to_string();
            }
        }

        // Then draw placed words
        for w in &self.words {
            if !w.placed {
                continue;
            }

            if w.is_solution() {
                continue;
            }

            for (x, y, c) in w {
                v[y*width + x] = c.to_string();
            }
        }

        // In the end, draw solution using a different color
        for w in &self.words {
            if !w.placed {
                continue;
            }

            if !w.is_solution() {
                continue;
            }

            for (x, y, c) in w {
                if self.colors {
                    v[y*width + x] = c.to_string().green().to_string();
                }
                else {
                    v[y*width + x] = c.to_string().bold().to_string();
                }
            }
        }

        format!("{}", String::from_iter(v))
    }
}
