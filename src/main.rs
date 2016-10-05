extern crate rand;
extern crate regex;
#[macro_use]
extern crate lazy_static;
extern crate discord as discord_lib;
extern crate retry;

use std::collections::{HashSet, HashMap};
use std::collections::hash_map::Entry;
use std::io::{BufRead, BufReader};
use std::fs::OpenOptions;
use std::hash::{Hash, SipHasher, Hasher};
use std::sync::{RwLock, Arc};
use std::sync::mpsc::{channel, Sender};
use std::io::{LineWriter, Write};

use rand::Rng;
use rand::Isaac64Rng;
use rand::SeedableRng;

mod discord;

const NONE: u32 = 0;
const END: u32 = 1;
const NAME: &'static str = "sbnkalny";
const OWNER: &'static str = "Mraof";

lazy_static! {
    static ref URL_REGEX: regex::Regex = regex::Regex::new(r"^http:|https:").unwrap();
}

fn main() {
    let words = Arc::new(RwLock::new(WordMap::new()));
    let instant = std::time::Instant::now();
    let markov_chain = MarkovChain::new(words.clone(), "lines/lines.txt");
    let load_time = instant.elapsed();
    println!("Loaded in {}.{:09} seconds",
             load_time.as_secs(),
             load_time.subsec_nanos());
    let (sender, thread) = markov_chain.thread();
    let discord_sender = discord::start(sender.clone(), words.clone());
    let (commander, commands) = channel();
    let console_thread = {
            let commander = commander.clone();
            std::thread::Builder::new().name("prompt".to_string()).spawn(move || {
                let (replier, reciever) = channel();
                let stdin = std::io::stdin();
                for line in stdin.lock().lines() {
                    let line = line.unwrap();
                    commander.send((line.clone(), replier.clone())).unwrap();
                    println!("{}", reciever.recv().unwrap());
                    if line.to_lowercase() == "stop" {
                        break;
                    }
                }
            })
        }
        .unwrap();
    while let Ok((command, replier)) = commands.recv() {
        let mut values = command.split(' ');
        replier.send(match values.next().unwrap_or("").to_lowercase().as_ref() {
                "reply" => {
                    let (replier, reciever) = channel();
                    sender.send(ChainMessage::Reply(values.map(|string| string.to_string())
                                                      .collect::<Vec<String>>()
                                                      .join(" "),
                                                  NAME.into(),
                                                  OWNER.into(),
                                                  vec![],
                                                  replier))
                        .unwrap();
                    reciever.recv().unwrap()
                }
                "m" => {
                    let (replier, reciever) = channel();
                    sender.send(ChainMessage::Command(values.map(|string| string.to_string())
                                                        .collect::<Vec<String>>()
                                                        .join(" "),
                                                    replier))
                        .unwrap();
                    reciever.recv().unwrap()
                }
                "stop" => {
                    sender.send(ChainMessage::Stop).unwrap();
                    replier.send("Stopped".to_string()).unwrap();
                    discord_sender.send("stop".to_string()).unwrap();
                    break;
                }
                _ => "".into(),
            })
            .unwrap();
    }
    thread.join().unwrap();
    console_thread.join().unwrap();
}

fn replace_names(string: &str, names: &Vec<String>) -> String {
    let regexes: Vec<regex::Regex> = names.clone()
        .into_iter()
        .map(|name| regex::Regex::new(&format!("{}{}{}", r"\b", regex::quote(&name.to_lowercase()), r"\b")).unwrap())
        .collect();
    let mut words = Vec::new();
    for word in string.split(' ') {
        let mut index = regexes.len();
        for i in 0..index {
            if regexes[i].is_match(&word.to_lowercase()) {
                index = i;
                break;
            }
        }
        if index != regexes.len() {
            words.push(regexes[index]
                .replace_all(&word.to_lowercase(),
                             regex::NoExpand(&format!("@name{}@", index)))
                .to_string());
        } else {
            words.push(word.to_string())
        }
    }
    words.join(" ")
}

fn lowercase_nonurl(string: &str) -> String {
    string.split(' ')
        .map(|word| {
            if URL_REGEX.is_match(word) {
                word.into()
            } else {
                word.to_lowercase()
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

#[derive(Default)]
pub struct WordMap {
    words: Vec<String>,
    ids: HashMap<String, u32>,
}

pub struct MarkovChain {
    next: HashMap<[u32; 3], Vec<u32>>,
    prev: HashMap<[u32; 3], Vec<u32>>,
    words: Arc<RwLock<WordMap>>,
    lines: HashSet<u64>,
    hasher: SipHasher,
    random: Isaac64Rng,
    filename: String,
    parent: Option<Sender<ChainMessage>>,
    tell_parent: bool,
}

pub enum ChainMessage {
    Learn(String, Vec<String>),
    Reply(String, String, String, Vec<String>, Sender<String>),
    NextNum([u32; 3], u8, Sender<usize>),
    Next([u32; 3], u8, Sender<Option<Vec<u32>>>),
    Command(String, Sender<String>),
    Stop,
}

// noinspection SpellCheckingInspection
impl MarkovChain {
    pub fn new(words: Arc<RwLock<WordMap>>, filename: &str) -> MarkovChain {
        let mut markov_chain = MarkovChain {
            words: words.clone(),
            random: Isaac64Rng::from_seed(&[rand::thread_rng().next_u64()]),
            next: Default::default(),
            prev: Default::default(),
            lines: Default::default(),
            hasher: Default::default(),
            filename: filename.to_string(),
            parent: None,
            tell_parent: true,
        };
        let path = std::path::Path::new(filename);
        std::fs::create_dir_all(&path.parent().unwrap()).unwrap();
        let file = OpenOptions::new().append(true).read(true).create(true).open(&path).unwrap();
        let reader = BufReader::new(file);
        let mut words = words.write().unwrap();
        for line in reader.lines() {
            let line = line.unwrap();
            line.hash(&mut markov_chain.hasher);
            let hash = markov_chain.hasher.finish();
            markov_chain.lines.insert(hash);
            markov_chain.learn_line(&line, &mut words);
        }
        markov_chain
    }

    pub fn thread(mut self) -> (Sender<ChainMessage>, std::thread::JoinHandle<()>) {
        let (sender, reciever) = channel();
        let thread = std::thread::Builder::new()
            .name(self.filename.clone())
            .spawn(move || {
                let file = OpenOptions::new().append(true).create(true).open(&self.filename).unwrap();
                let mut writer = LineWriter::new(file);
                while let Ok(message) = reciever.recv() {
                    match message {
                        ChainMessage::Learn(line, names) => {
                            if self.add_line(&line, &names) {
                                writer.write(format!("{}\n", line).as_bytes()).unwrap();
                            }
                        }
                        ChainMessage::Reply(line, name, sender, names, replier) => {
                            replier.send(self.reply(&line, &name, &sender, &names)).unwrap();
                        }
                        ChainMessage::NextNum(key, dir, sender) => {
                            sender.send(if dir == 0 {
                                    self.prev.get(&key).unwrap_or(&vec![]).len()
                                } else {
                                    self.next.get(&key).unwrap_or(&vec![]).len()
                                })
                                .unwrap();
                        }
                        ChainMessage::Next(key, dir, sender) => {
                            sender.send(self.next(&key, dir)).unwrap();
                        }
                        ChainMessage::Command(command, sender) => {
                            sender.send(self.command(&command)).unwrap();
                        }
                        ChainMessage::Stop => break,
                    }
                }
            })
            .unwrap();
        (sender, thread)
    }

    pub fn add_line(&mut self, line: &str, names: &Vec<String>) -> bool {
        let line = replace_names(line, names);
        line.hash(&mut self.hasher);
        let hash = self.hasher.finish();
        if self.tell_parent {
            if let Some(ref sender) = self.parent {
                sender.send(ChainMessage::Learn(line.clone(), Vec::new())).unwrap();
            }
        }
        if !self.lines.contains(&hash) {
            self.lines.insert(hash);
            let words = self.words.clone();
            let mut words = words.write().unwrap();
            self.learn_line(&line, &mut words);
            true
        } else {
            false
        }
    }

    pub fn learn_line(&mut self, line: &str, words: &mut WordMap) {
        let mut line_words: Vec<u32> = line.split(' ').map(|word| words.lookup(word.into())).collect();
        line_words.insert(0, END);
        line_words.insert(0, END);
        line_words.push(END);
        line_words.push(END);
        let mut prev3;
        let mut prev2;
        let mut prev1;
        let mut next3;
        let mut next2;
        let mut next1;
        let end = line_words.len() - 2;
        for i in 2..end {
            if i > 2 {
                next3 = [line_words[i - 3], line_words[i - 2], line_words[i - 1]];
                next2 = [next3[1], next3[2], NONE];
                next1 = [next3[2], NONE, NONE];
                self.next.entry(next3).or_insert(Default::default()).push(line_words[i]);
                self.next.entry(next2).or_insert(Default::default()).push(line_words[i]);
                self.next.entry(next1).or_insert(Default::default()).push(line_words[i]);
            }
            if i < end {
                prev3 = [line_words[i + 2], line_words[i + 1], line_words[i]];
                prev2 = [prev3[1], prev3[2], NONE];
                prev1 = [prev3[2], NONE, NONE];
                self.prev.entry(prev3).or_insert(Default::default()).push(line_words[i - 1]);
                self.prev.entry(prev2).or_insert(Default::default()).push(line_words[i - 1]);
                self.prev.entry(prev1).or_insert(Default::default()).push(line_words[i - 1]);
            }
        }
    }

    pub fn reply(&mut self, input: &str, name: &str, sender: &str, names: &Vec<String>) -> String {
        let limit = "\0randomsentence\0" != sender;

        let mut names = names.clone();
        let mut reply = String::new();
        if names.is_empty() {
            names.push(name.clone().into());
            names.push(sender.clone().into());
        }
        let input = replace_names(&input, &names);
        lazy_static! {
            static ref SENTENCE_END: regex::Regex = regex::Regex::new(r"\. |\n").unwrap();
        }
        let mut inputs: Vec<String> = SENTENCE_END.split(&input).map(|input| input.to_string()).collect();
        let input = inputs.pop().unwrap();
        if !inputs.is_empty() {
            reply = inputs.iter()
                .map(|input| self.reply(&input, &name, &sender, &names))
                .collect::<Vec<String>>()
                .join(". ");
        }


        let input: Vec<u32> = {
            let mut words = self.words.write().unwrap();
            lowercase_nonurl(&input).split(' ').map(|word| words.lookup(word.into())).collect()
        };

        if input.is_empty() {
            return reply;
        }

        let mut best = [input[0], NONE, NONE];
        let mut temp_keys = vec![[END, NONE, NONE], [END, END, NONE], [END, END, END]];
        let mut best_size = self.next.get(&best).unwrap_or(&vec![]).len();
        for word in &input {
            temp_keys[2] = [temp_keys[2][1], temp_keys[2][2], *word];
            temp_keys[1] = [temp_keys[2][1], temp_keys[2][2], NONE];
            temp_keys[0] = [temp_keys[2][2], NONE, NONE];
            for key in &temp_keys {
                if key[0] != END && key[1] != END && key[2] != END {
                    let temp_size = self.next.get(key).unwrap_or(&vec![]).len();
                    if (temp_size > 0) && (best_size == 0 || (temp_size < best_size)) {
                        best = *key;
                        best_size = temp_size;
                    }
                }
            }
        }

        let mut sentence = vec![best[0], best[1], best[2]];
        sentence.retain(|&word| word > END);

        for dir in 0..2 {
            let mut words_temp: HashMap<[u32; 3], Vec<u32>> = HashMap::new();
            let mut last_word = match if dir == 0 {
                sentence.get(1)
            } else {
                sentence.get((sentence.len() as isize - 2) as usize)
            } {
                Some(&word) => word,
                None => END,
            };
            let mut last_word2 = match if dir == 0 {
                sentence.get(2)
            } else {
                sentence.get((sentence.len() as isize - 3) as usize)
            } {
                Some(&word) => word,
                None => END,
            };

            let mut size = sentence.len() - 1;
            while size < sentence.len() {
                size = sentence.len();
                let current_word = if dir == 0 {
                    sentence[0]
                } else {
                    sentence[sentence.len() - 1]
                };
                let keys = vec![[current_word, NONE, NONE], [last_word, current_word, NONE], [last_word2, last_word, current_word]];
                let mut word = END;
                for key in &keys {
                    if !words_temp.contains_key(key) {
                        if let Some(list) = self.next(key, dir) {
                            words_temp.insert(*key, list);
                        }
                    }
                }
                let key_index = {
                    match words_temp.get(&keys[1]).unwrap_or(&vec![]).len() {
                        0 => {
                            if (input.len() as f32) / (sentence.len() as f32) < self.random.next_f32() {
                                break;
                            }
                            0
                        }
                        two_length => {
                            if words_temp.get(&keys[2]).unwrap_or(&vec![]).len() > 0 && self.random.next_f32() > 4.0 / (two_length as f32) {
                                2
                            } else {
                                1
                            }
                        }
                    }

                };
                if let Some(mut list) = words_temp.get_mut(&keys[key_index]) {
                    let mut index = self.random.next_u32() as usize % list.len();
                    if limit {
                        for _ in input.len()..sentence.len() {
                            index = self.random.next_u32() as usize % list.len();
                            if let Some(word) = list.get(index) {
                                if word == &END {
                                    break;
                                }
                            }
                        }
                    }
                    word = list.remove(index);
                }
                if word != END {
                    if dir == 0 {
                        sentence.insert(0, word);
                    } else {
                        sentence.push(word);
                    }
                }
                last_word2 = last_word;
                last_word = current_word;
            }
        }


        let mut name_replacements: HashMap<String, String> = HashMap::new();
        lazy_static! {
            static ref NAME_TOKEN: regex::Regex = regex::Regex::new(r"@name\d+@").unwrap();
        }

        let words = self.words.read().unwrap();
        let mut random = self.random;
        let mut sentence = sentence.iter()
            .map(|&word| {
                NAME_TOKEN.replace_all(&words.get(word), |caps: &regex::Captures| {
                    name_replacements.entry(caps[0].to_string())
                        .or_insert(names[random.next_u32() as usize % names.len()].clone())
                        .clone()
                })
            })
            .collect::<Vec<String>>()
            .join(" ");

        if !URL_REGEX.is_match(&sentence) {
            let first = sentence.remove(0).to_uppercase().next().unwrap();
            sentence.insert(0, first);
        }

        reply + &sentence
    }

    pub fn next(&mut self, key: &[u32; 3], dir: u8) -> Option<Vec<u32>> {
        match if dir == 0 {
            self.prev.get(key)
        } else {
            self.next.get(key)
        } {
            Some(list) => {
                match self.parent {
                    Some(ref parent) => {
                        let (sender, reciever) = channel();
                        parent.send(ChainMessage::NextNum(*key, dir, sender)).unwrap();
                        let num = reciever.recv().unwrap();
                        if num > 0 && self.random.next_u32() % num as u32 > self.random.next_u32() % (list.len() * list.len()) as u32 {
                            let (sender, reciever) = channel();
                            parent.send(ChainMessage::Next(*key, dir, sender)).unwrap();
                            reciever.recv().unwrap()
                        } else {
                            Some(list.clone())
                        }
                    }
                    None => Some(list.clone()),
                }
            }
            None => {
                match self.parent {
                    Some(ref parent) => {
                        let (sender, reciever) = channel();
                        parent.send(ChainMessage::Next(*key, dir, sender)).unwrap();
                        reciever.recv().unwrap()
                    }
                    None => None,
                }
            }
        }
    }

    pub fn random_sentence(&mut self, names: &Vec<String>) -> String {
        let start = {
            let offset = self.random.next_u32() as usize % self.next.len();
            let empty_key = [END, END, END];
            let key = self.next.keys().skip(offset).next().unwrap_or(&empty_key);
            let mut sentence = vec![key[0], key[1], key[2]];
            sentence.retain(|&word| word > END);
            let words = self.words.read().unwrap();
            sentence.iter().map(|&word| words.get(word)).collect::<Vec<String>>().join(" ")
        };
        self.reply(&start, "", "\0randomsentence\0", names)
    }

    pub fn command(&mut self, command: &str) -> String {
        let parts: Vec<String> = lowercase_nonurl(command).split(' ').map(|part| part.to_string()).collect();
        match parts.get(0).map(String::as_ref) {
            Some("stats") => {
                let words = self.words.read().unwrap();
                format!("Forwards keys: {}, Backwards keys: {}, words: {}, lines: {}",
                        self.next.len(),
                        self.prev.len(),
                        words.len(),
                        self.lines.len())
            }
            Some("wordstats") => {
                let keys = {
                    let mut prev = Vec::with_capacity(3);
                    let mut next = Vec::with_capacity(3);
                    let mut unknown = false;
                    {
                        let words = self.words.read().unwrap();
                        for i in 0..3 {
                            match parts.get(i + 1) {
                                Some(ref word) => {
                                    if let Some(word) = words.find(&word) {
                                        next.push(*word);
                                        prev.push(*word);
                                    } else {
                                        unknown = true;
                                        break;
                                    }
                                }
                                None => {
                                    next.push(NONE);
                                    prev.insert(0, NONE)
                                }
                            }
                        }
                    }
                    if unknown {
                        return self.reply("I don't know those words", "", "", &parts);
                    }
                    prev.reverse();
                    [[prev[0], prev[1], prev[2]], [next[0], next[1], next[2]]]
                };
                struct Stats {
                    best: u32,
                    count: usize,
                    best_count: usize,
                };
                let mut stats = [Stats {
                                     best: END,
                                     count: 0,
                                     best_count: 0,
                                 },
                                 Stats {
                                     best: END,
                                     count: 0,
                                     best_count: 0,
                                 }];
                let maps = [&self.prev, &self.next];
                for dir in 0..2 {
                    if let Some(list) = maps[dir].get(&keys[dir]) {
                        let mut counts = HashMap::new();
                        let mut list = list.clone();
                        list.retain(|&word| word > END);
                        for word in list {
                            *counts.entry(word).or_insert(0) += 1
                        }
                        stats[dir].count = counts.len();
                        for entry in counts {
                            if entry.1 > stats[dir].best_count {
                                stats[dir].best_count = entry.1;
                                stats[dir].best = entry.0;
                            }
                        }
                    }
                }
                let words = self.words.read().unwrap();
                let mut key = keys[1].to_vec();
                key.retain(|&word| word > END);
                let key = key.iter().map(|&word| words.get(word)).collect::<Vec<String>>().join(" ");
                format!("{} has {}, {}",
                        key,
                        if stats[0].best == END {
                            "no previous words".to_string()
                        } else {
                            format!("{} previous words, most often \"{}\" ({} times)",
                                    stats[0].count,
                                    words.get(stats[0].best),
                                    stats[0].best_count)
                        },
                        if stats[0].best == END {
                            "no next words".to_string()
                        } else {
                            format!("{} next words, most often \"{}\" ({} times)",
                                    stats[0].count,
                                    words.get(stats[0].best),
                                    stats[0].best_count)
                        })
            }
            Some("sentence") => self.random_sentence(&vec![NAME.to_string(), OWNER.to_string()]),
            _ => "".into(),
        }
    }
}

impl WordMap {
    pub fn new() -> WordMap {
        let mut word_map: WordMap = Default::default();
        word_map.lookup("\0".into());
        word_map.lookup("".into());
        word_map
    }

    pub fn lookup(&mut self, word: String) -> u32 {
        match self.ids.entry(word.clone()) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => {
                self.words.push(word.clone());
                entry.insert(self.words.len() as u32 - 1);
                self.words.len() as u32 - 1
            }
        }
    }

    /// Look up word, but don't add it if not found
    pub fn find(&self, word: &str) -> Option<&u32> {
        self.ids.get(word)
    }

    pub fn get(&self, id: u32) -> String {
        self.words[id as usize].clone()
    }

    pub fn len(&self) -> usize {
        self.words.len()
    }
}
