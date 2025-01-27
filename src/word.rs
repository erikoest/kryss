use std::fmt::{Formatter, Result, Display};

#[derive(PartialEq, Clone)]
pub enum Orientation {
    Right,
    Left,
    Down,
    Up,
}

impl Orientation {
    pub fn is_horizontal(&self) -> bool {
        return match self {
            Orientation::Right | Orientation::Left => true,
            _ => false,
        };
    }

    pub fn is_vertical(&self) -> bool {
        return match self {
            Orientation::Down | Orientation::Up => true,
            _ => false,
        };
    }

    pub fn is_reversed(&self) -> bool {
        return match self {
            Orientation::Left | Orientation::Up => true,
            _ => false,
        };
    }

    pub fn same_or_opposite_direction(&self, other: &Orientation) -> bool {
        return (self.is_horizontal() && other.is_horizontal()) ||
            (self.is_vertical() && other.is_vertical());
    }
}

impl Display for Orientation {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", match self {
            Orientation::Right => 'R',
            Orientation::Left  => 'L',
            Orientation::Down  => 'D',
            Orientation::Up    => 'U',
        })
    }
}

#[derive(Clone)]
pub struct Word {
    pub o: Orientation,
    pub x: usize,
    pub y: usize,
    pub length: usize,
    pub key: Option<String>,
    pub candidates: Vec<String>,
    pub placed: bool,
}

impl Word {
    pub fn from_parts(parts: &[&str]) -> Self {
        let o = match parts[0] {
            "R" => Orientation::Right,
            "L" => Orientation::Left,
            "D" => Orientation::Down,
            "U" => Orientation::Up,
            inv => panic!("Invalid orientation {}", inv)
        };

        let x = parts[1].parse().unwrap();
        let y = parts[2].parse().unwrap();
        let len_part = parts[3];
        let (length, candidates, key);

        if parts.len() > 4 {
            length = len_part.parse().unwrap();
            let key_part = parts[4].to_string();

            if let Some(i) = key_part.find('=') {
                candidates = vec![key_part[i + 1..key_part.len()].to_string()];
                key = Some(key_part[0..i].to_string());
            }
            else {
                candidates = vec!();
                key = Some(key_part);
            }
        }
        else {
            key = None;

            if let Some(i) = len_part.find('=') {
                candidates = vec![len_part[i + 1..len_part.len()].to_string()];
                length = len_part[0..i].to_string().parse().unwrap();
            }
            else {
                candidates = vec!();
                length = len_part.parse().unwrap();
            }
        }

        let placed = !candidates.is_empty();

        Self {
            o: o,
            x: x,
            y: y,
            length: length,
            key: key,
            candidates: candidates,
            placed: placed,
        }
    }

    pub fn char_at(&self, ix: usize) -> char {
        assert!(self.has_one_candidate());

        self.candidates[0].chars().nth(ix).unwrap()
    }

    pub fn place(&mut self, opt_word: Option<String>) {
        // Mark word as placed
        self.placed = true;

        if let Some(word) = opt_word {
            self.candidates.clear();
            self.candidates.push(word);
        }
    }

    pub fn unplace(&mut self) {
        self.placed = false;
        self.candidates.clear();
    }

    pub fn has_one_candidate(&self) -> bool {
        return self.candidates.len() == 1;
    }

    pub fn has_candidates(&self) -> bool {
        return !self.candidates.is_empty();
    }

    pub fn is_conflicting(&self, b: &Word) -> bool {
        // Crossing words are not conflicting
        if self.is_crossing(b) {
            return false;
        }

        // Non-crossing words must be apart from each other with at least one
        // field between them. The exception is that corners may touch.
        let (a_xmin, a_xmax) = (self.xmin(), self.xmax());
        let (a_ymin, a_ymax) = (self.ymin(), self.ymax());
        let (b_xmin, b_xmax) = (b.xmin(), b.xmax());
        let (b_ymin, b_ymax) = (b.ymin(), b.ymax());

        let b_is_over = b_ymax < a_ymin;
        let b_is_under = b_ymin > a_ymax;
        let b_is_left = b_xmax < a_xmin;
        let b_is_right = b_xmin > a_xmax;

        // Check corners
        if (b_is_over && (b_is_left || b_is_right)) ||
            (b_is_under && (b_is_left || b_is_right)) {
            return false;
        }

        // Check sides
        if b_xmax + 1 < a_xmin || b_xmin - 1 > a_xmax ||
            b_ymax + 1 < a_ymin || b_ymin - 1 > a_ymax {
            return false;
        }

        // Allow same directional words to be adjacent
        if self.o.is_horizontal() && b.o.is_horizontal() {
            if self.y != b.y {
                return false;
            }
        }
        else if self.o.is_vertical() && b.o.is_vertical() {
            if self.x != b.x {
                return false;
            }
        }

        return true;
    }

    pub fn is_crossing(&self, word: &Word) -> bool {
        // Words in the same or oposite direction are not crossing
        if self.o.same_or_opposite_direction(&word.o) {
            return false;
        }

        if self.o.is_horizontal() {
            if self.xmin() <= word.xmin() && self.xmax() >= word.xmax() &&
                self.ymin() >= word.ymin() && self.ymax() <= word.ymax() {
                return true;
            }
        }
        else {
            if self.xmin() >= word.xmin() && self.xmax() <= word.xmax() &&
                self.ymin() <= word.ymin() && self.ymax() >= word.ymax() {
                return true;
            }
        }

        return false;
    }

    pub fn xmin(&self) -> usize {
        match self.o {
            Orientation::Right | Orientation::Down | Orientation::Up => {
                return self.x;
            },
            Orientation::Left => {
                return self.x - self.length + 1;
            },
        }
    }

    pub fn ymin(&self) -> usize {
        match self.o {
            Orientation::Right | Orientation::Left | Orientation::Down => {
                return self.y;
            },
            Orientation::Up => {
                return self.y - self.length + 1;
            },
        }
    }

    pub fn xmax(&self) -> usize {
        match self.o {
            Orientation::Right => {
                return self.x + self.length - 1;
            },
            Orientation::Left | Orientation::Down | Orientation::Up => {
                return self.x;
            },
        }
    }

    pub fn ymax(&self) -> usize {
        match self.o {
            Orientation::Right | Orientation::Left | Orientation::Up => {
                return self.y;
            },
            Orientation::Down => {
                return self.y + self.length - 1;
            },
        }
    }

    pub fn is_missing(&self) -> bool {
        return self.candidates.is_empty();
    }

    pub fn is_ambiguous(&self) -> bool {
        return self.candidates.len() > 1;
    }

    pub fn is_solution(&self) -> bool {
        return self.key.is_none();
    }

    pub fn position_at_index(&self, i: usize) -> (usize, usize) {
        return match self.o {
            Orientation::Right => (self.x + i, self.y),
            Orientation::Left  => (self.x - i, self.y),
            Orientation::Down  => (self.x, self.y + i),
            Orientation::Up    => (self.x, self.y - i),
        };
    }

    pub fn position_in_word(&self, x: usize, y: usize) -> bool {
        return x >= self.xmin()
            && x <= self.xmax()
            && y >= self.ymin()
            && y <= self.ymax();
    }

    fn iter(&self) -> WordIter {
        WordIter::new(self)
    }
}

impl<'a> IntoIterator for &'a Word {
    type Item = (usize, usize, char);

    type IntoIter = WordIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl ToString for Word {
    fn to_string(&self) -> String {
        if let Some(k) = &self.key {
            if self.placed {
                return format!("{},{},{},{},{}={}", self.o, self.x, self.y,
                               self.length, k, self.candidates[0]);
            }
            else {
                return format!("{},{},{},{},{}", self.o, self.x, self.y,
                               self.length, k);
            }
        }
        else {
            if self.placed {
                return format!("{},{},{},{}={}", self.o, self.x, self.y,
                               self.length, self.candidates[0]);
            }
            else {
                return format!("{},{},{},{}", self.o, self.x, self.y,
                               self.length);
            }
        }
    }
}

pub struct WordIter<'a> {
    index: usize,
    word: &'a Word,
}

impl<'a> WordIter<'a> {
    fn new(word: &'a Word) -> Self {
        Self {
            index: 0,
            word: word,
        }
    }
}

impl<'a> Iterator for WordIter<'a> {
    type Item = (usize, usize, char);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.word.length {
            return None;
        }

        let i = self.index;
        let c;

        if self.word.placed {
            c = self.word.char_at(i);
        }
        else {
            c = '.';
        }

        let (x, y) = self.word.position_at_index(i);

        self.index += 1;

        return Some((x, y, c));
    }
}
