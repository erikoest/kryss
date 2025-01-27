use url::Url;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::fs;
use sxd_html::parse_html;
use sxd_xpath::{Value, evaluate_xpath};

#[derive(Serialize, Deserialize)]
pub struct Dictionary {
    words: HashMap<String, HashMap<usize, Vec<String>>>,
    #[serde(skip_serializing, skip_deserializing)]
    pub changed: bool,
    #[serde(skip_serializing, skip_deserializing)]
    pub filename: String,
}

impl Dictionary {
    pub fn new() -> Self {
        Self {
            words: HashMap::new(),
            changed: false,
            filename: "".to_string(),
        }
    }

    pub fn from_file(file: &str) -> Self {
        let data = fs::read_to_string(&file).expect("Unable to read file");
        let mut ret: Self = serde_json::from_str(&data).unwrap();
        ret.filename = file.to_string();
        ret.changed = false;

        return ret;
    }

    pub fn write_to_file(&mut self, opt_fname: Option<&str>) {
        let mut filename = self.filename.clone();

        if let Some(fname) = opt_fname {
            filename = fname.to_string();
        }

        let data = serde_json::to_string(&self).unwrap();
        fs::write(&filename, data).expect("Unable to write file");

        self.changed = false;
        self.filename = filename;
    }

    pub fn add_word(&mut self, key: &str, word: &str) {
        let len = word.chars().count();

        if key.find("xxxx").is_some() {
            println!("Don't add unknown word {}", key);
        }

        if let Some(whash) = self.words.get_mut(key) {
            if let Some(words) = whash.get_mut(&len) {
                // Key exists for the word length. First, check if word
                // is already registered.
                for w in &mut *words {
                    if w == word {
                        return;
                    }
                }

                // Word is not registered. Add it
                println!("Pushing word to dictionary");
                words.push(word.to_string());
            }
            else {
                whash.insert(len, vec![word.to_string()]);
            }
        }
        else {
            let mut whash = HashMap::new();
            whash.insert(len, vec![word.to_string()]);
            self.words.insert(key.to_string(), whash);
        }

        self.changed = true;
    }

    fn lookup_from_gratiskryss(&mut self, key: &str) {
        if key.find("xxxx").is_some() {
            println!("Skip looking up unknown word {}", key);
            return;
        }

        println!("Looking up {} from gratiskryssord", key);
        let mut words: HashMap<usize, Vec<String>> = HashMap::new();

        if key.find("xxxx").is_some() {
            // Don't look up unknown word
            return;
        }

        let mut url = Url::parse("https://www.gratiskryssord.no/kryssordbok/")
            .unwrap().join(key).unwrap();
        loop {
            let html = reqwest::blocking::get(url.as_str())
                .unwrap().text().unwrap();
            let package = parse_html(&html);
            let doc = package.as_document();
            let val = evaluate_xpath(&doc, "/html/body/section/div/div/div[1]/article/div[*]/div[*]/div[*]/div[*]/div[*]/div[*]/section/ul/li[*]/a/text()").unwrap();

            match val {
                Value::Nodeset(ns) => {
                    for n in ns {
                        let word = n.string_value().trim().to_string();

                        if word.contains(" ") {
                            continue;
                        }

                        let length = word.chars().count();
                        if !words.contains_key(&length) {
                            words.insert(length, vec!());
                        }
                        words.get_mut(&length).unwrap().push(word);
                    }
                }
                _ => {
                    panic!("Expected nodeset");
                }
            }

            let val = evaluate_xpath(&doc, "/html/body/section/div/div/div[1]/article/div[3]/div/form/div[1]/div[2]/ul/li[last()]/@ng-init").unwrap();

            match val {
                Value::Nodeset(ns) => {
                    let opt_next = ns.into_iter().next();

                    if let Some(next_node) = opt_next {
                        let next = next_node
                            .string_value()
                            .strip_prefix("shFunc.setNextLink('").unwrap()
                            .replace("');", "");

                        if next == "" {
                            break;
                        }
                        url = url.join(&next).unwrap();
                    }
                    else {
                        break;
                    }
                }
                _ => {
                    panic!("Expected string");
                }
            }
        }

        self.words.insert(key.to_string(), words);
        self.changed = true;
    }

    pub fn lookup(&mut self, key: &str, length: usize, opt_hint: Option<&str>)
                  -> Vec<String> {
        let hint;

        if let Some(h) = opt_hint {
            hint = h.to_string();
        }
        else {
            hint = String::from_iter(vec!['.'; length]);
        }

        if !self.words.contains_key(key) {
            self.lookup_from_gratiskryss(key);
        }

        let mut ret = vec!();

        if self.words.contains_key(key) {
            if self.words[key].contains_key(&length) {
                'outer: for w in &self.words[key][&length] {
                    for (a, b) in w.chars().zip(hint.chars()) {
                        if a != b && b != '.' {
                            continue 'outer;
                        }
                    }

                    ret.push(w.to_string());
                }
            }
        }

        return ret;
    }

    pub fn to_string(&self) -> String {
        return serde_json::to_string(&self).unwrap();
    }
}
