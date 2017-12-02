extern crate rand;
extern crate regex;
#[macro_use]
extern crate lazy_static;
extern crate discord as discord_lib;
extern crate retry;
extern crate hyper;
extern crate hyper_tls;
extern crate tokio_core;
extern crate futures;
extern crate egg_mode;
extern crate tumblr as tumblr_lib;
extern crate tcpgen;
extern crate morbitgen;
extern crate linked_hash_map;
extern crate unicode_normalization;
extern crate distance;
extern crate byteorder;
#[macro_use]
extern crate ordermap;

#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate bincode;

extern crate meval;
extern crate astro;
extern crate chrono;

use std::collections::{BTreeSet, HashMap};
use std::collections::hash_map::Entry;
use std::io::{BufRead, BufReader};
use std::fs::{OpenOptions, File};
use std::path::Path;
use std::hash::{Hash, Hasher, BuildHasher};
use std::collections::hash_map::RandomState;
use std::sync::{RwLock, Arc};
use std::sync::mpsc::{channel, Sender};
use std::io::{LineWriter, Write, BufWriter, Read};

use rand::Rng;
use rand::OsRng;
use rand::distributions::{IndependentSample, Weighted, WeightedChoice};

use bincode::{serialize_into, deserialize_from, Infinite};
use byteorder::{WriteBytesExt, ByteOrder, BE};

use ordermap::OrderMap;

mod discord;
mod twitter;
mod tumblr;
mod astronomy;

type Key = [u32; 3];

const NONE: u32 = !0;
const END: u32 = !1;

lazy_static! {
    static ref URL_REGEX: regex::Regex = regex::Regex::new(r"^http:|https:").expect("Failed to make URL_REGEX");
    static ref SENTENCE_END: regex::Regex = regex::Regex::new("\\. |\n").expect("Failed to make SENTENCE_END");
    static ref CONFIG: GlobalConfig = GlobalConfig::load("config/config.json");
}

//TODO Save cache of lines as u32 and word map
fn main() {
    let word_path = Path::new("cache/words");
    let words = if let Ok(mut word_file) = File::open(word_path) {
        let mut words: WordMap = deserialize_from(&mut word_file, Infinite).expect("Unable to read cache/words, please delete it");
        words.cached = true;
        words
    } else {
        WordMap::new()
    };
    let words = Arc::new(RwLock::new(words));
    let instant = std::time::Instant::now();
    let markov_chain = MarkovChain::new(words.clone(), "lines/lines.txt");
    let load_time = instant.elapsed();
    println!("Loaded in {}.{:09} seconds",
             load_time.as_secs(),
             load_time.subsec_nanos());
    let (sender, thread) = markov_chain.thread();
    let mut chats = HashMap::new();
    if CONFIG.chats.contains("discord") {
        chats.insert("discord", discord::start(sender.clone(), words.clone()));
    }
    if CONFIG.chats.contains("twitter") {
        chats.insert("twitter", twitter::start(sender.clone(), words.clone()));
    }
    if CONFIG.chats.contains("tumblr") {
        chats.insert("tumblr", tumblr::start(sender.clone(), words.clone()));
    }
    println!("Chats loaded: {:?}", CONFIG.chats);
    let (commander, commands) = channel();
    let console_thread = {
        let commander = commander.clone();
        std::thread::Builder::new().name("prompt".to_string()).spawn(move || {
            let (replier, receiver) = channel();
            let stdin = std::io::stdin();
            for line in stdin.lock().lines() {
                let line = line.expect("Failed to unwrap command line");
                commander.send((line.clone(), replier.clone())).expect("Failed to send command");
                println!("{}",
                         receiver.recv().expect("Failed to receive command response"));
                if line.to_lowercase() == "stop" {
                    break;
                }
            }
        })
    }
        .expect("Failed to make prompt thread");
    while let Ok((command, replier)) = commands.recv() {
        let mut values = command.split(' ');
        replier.send(match values.next().unwrap_or("").to_lowercase().as_ref() {
            "reply" => {
                let (replier, receiver) = channel();
                sender.send(ChainMessage::Reply(values.map(|string| string.to_string())
                                                    .collect::<Vec<String>>()
                                                    .join(" "),
                                                CONFIG.name.clone(),
                                                CONFIG.owner.clone(),
                                                Vec::new(),
                                                replier))
                    .expect("Failed to send reply message");
                receiver.recv().expect("Failed to receive reply")
            }
            "m" => {
                let (replier, receiver) = channel();
                sender.send(ChainMessage::Command(values.map(|string| string.to_string())
                                                      .collect::<Vec<String>>()
                                                      .join(" "),
                                                  Power::Cool,
                                                  replier))
                    .expect("Failed to send command message");
                receiver.recv().expect("Failed to receive command reply")
            }
            "stop" => {
                sender.send(ChainMessage::Stop).expect("Failed to send stop message");
                replier.send("Stopped".to_string()).expect("Failed to reply with \"Stopped\"");
                for (name, chat) in chats {
                    chat.send("stop".into()).unwrap_or_else(|_| println!("{} panicked at some point", name));
                }
                break;
            }
            command => {
                if let Some(chat) = chats.get(command) {
                    chat.send(values.collect::<Vec<_>>().join(" ")).expect("Can't send to chat");
                }
                "".into()
            }
        })
            .expect("Failed to reply");
    }
    thread.join().expect("Failed to join thread");
    console_thread.join().expect("Failed to join console thread");
    std::fs::create_dir_all(word_path.parent().expect("Failed to get parent")).expect("Failed to create cache directory");
    let mut word_file = File::create(word_path).expect("Unable to create cache/words");
    serialize_into(&mut word_file, &*words.read().unwrap(), Infinite).expect("Unable to write to cache/words");
}

fn replace_names(string: &str, names: &[String]) -> String {
    let regexes: Vec<regex::Regex> = names
        .into_iter()
        .map(|name| regex::Regex::new(&format!("{}{}{}", r"\b", regex::escape(&name.to_lowercase()), r"\b")).expect("Failed to create regex"))
        .collect();
    let mut words = Vec::new();
    for word in string.split(' ') {
        let mut index = regexes.len();
        for (i, regex) in regexes.iter().enumerate() {
            if regex.is_match(&word.to_lowercase()) {
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

#[derive(Default, Serialize, Deserialize)]
pub struct WordMap {
    words: Vec<OrderMap<String, u32>>,
    ids: HashMap<String, u32>,
    cached: bool,
}

#[derive(Default, Serialize, Deserialize)]
pub struct MarkovChainConfig {
    pub tell_parent: bool,
    pub consume: f32,
    pub strength: f32,
}

pub struct MarkovChain {
    pub core: Arc<RwLock<ChainCore>>,
    pub words: Arc<RwLock<WordMap>>,
    pub lines: BTreeSet<u64>,
    build_hasher: RandomState,
    pub filename: String,
    parent: Option<Sender<ChainMessage>>,
    pub tell_parent: bool,
}

pub enum ChainMessage {
    Learn(String, Vec<String>),
    Reply(String, String, String, Vec<String>, Sender<String>),
    NextNum(Key, u8, Sender<usize>),
    Next(Key, u8, Sender<Option<Vec<u32>>>),
    Command(String, Power, Sender<String>),
    RandomWord(Sender<String>),
    ChangeParent(Option<Sender<ChainMessage>>),
    Core(Sender<Arc<RwLock<ChainCore>>>),
    Stop,
}

// noinspection SpellCheckingInspection
impl MarkovChain {
    pub fn new(words: Arc<RwLock<WordMap>>, filename: &str) -> MarkovChain {
        let mut markov_chain = MarkovChain {
            core: Default::default(),
            words: words.clone(),
            lines: Default::default(),
            build_hasher: Default::default(),
            filename: filename.to_string(),
            parent: None,
            tell_parent: true,
        };
        {
            println!("Loading chain {}", filename);
            let mut words = words.write().expect("Failed to get words for writing");
            let cache_path = format!("cache/{}", filename);
            let cache_path = Path::new(&cache_path);
            let cache_file = if words.cached { File::open(cache_path).ok() } else { None };
            std::fs::create_dir_all(Path::new(&filename).parent().expect("Failed to get parent")).expect("Failed to create directory");
            std::fs::create_dir_all(cache_path.parent().expect("Failed to get parent")).expect("Failed to create cache directory");
            let lines: Vec<Vec<u32>> = if let Some(mut file) = cache_file {
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes).expect("Failed to read cache file");
                let mut lines = Vec::new();
                let mut line_words = Vec::new();
                for i in 0..bytes.len() / 4 {
                    let word = BE::read_u32(&bytes[i * 4..i * 4 + 4]);
                    if word == END {
                        lines.push(line_words);
                        line_words = Vec::new();
                    } else {
                        line_words.push(word);
                    }
                }
                lines
            } else if let Ok(file) = File::open(&filename) {
                let reader = BufReader::new(file);
                let mut bytes = Vec::new();
                let lines = reader.lines().map(|line| {
                    let line = line.expect("Failed to unwrap line");
                    let line_words = line.split(' ').map(|word| words.lookup(word.into())).collect::<Vec<u32>>();
                    for &word in &line_words {
                        //Just ignore the result because this can never fail for vecs
                        let _ = bytes.write_u32::<BE>(word);
                    }
                    let _ = bytes.write_u32::<BE>(END);
                    line_words
                }).collect();
                let cache_file = File::create(cache_path).expect("Failed to open cache file");
                let mut cache_writer = BufWriter::new(cache_file);
                cache_writer.write_all(&bytes).expect("Failed to write line cache");
                lines
            } else {
                Vec::new()
            };

            for line_words in lines {
                let mut core = markov_chain.core.write().expect("Couldn't lock core for writing");
                let mut hasher = markov_chain.build_hasher.build_hasher();
                line_words.hash(&mut hasher);
                let hash = hasher.finish();
                if markov_chain.lines.insert(hash) {
                    core.learn(line_words);
                }
            }
        }
        markov_chain
    }

    pub fn load_config(&mut self) {}

    pub fn thread(mut self) -> (Sender<ChainMessage>, std::thread::JoinHandle<()>) {
        let (sender, receiver) = channel();
        let filename = self.filename.clone();
        let thread = std::thread::Builder::new()
            .name(self.filename.clone())
            .spawn(move || {
                let file = OpenOptions::new().append(true).create(true).open(&self.filename).expect("Failed to open");
                let cache_file = OpenOptions::new().append(true).create(true).open(&format!("cache/{}", &self.filename)).expect("Failed to open cache file");
                let mut writer = LineWriter::new(file);
                let mut cache_writer = BufWriter::new(cache_file);
                let mut random = OsRng::new().unwrap();
                while let Ok(message) = receiver.recv() {
                    match message {
                        ChainMessage::Learn(line, names) => {
                            for (line, vec) in self.add_line(&line, &names) {
                                writer.write_all(format!("{}\n", line).as_bytes()).expect("Failed to write line");
                                let mut bytes: Vec<u8> = Vec::with_capacity((vec.len() + 1) * std::mem::size_of::<u32>());
                                for word in vec {
                                    //Just ignore the result because this can never fail for vecs
                                    let _ = bytes.write_u32::<BE>(word);
                                }
                                let _ = bytes.write_u32::<BE>(END);
                                cache_writer.write_all(&bytes).expect("Failed to write line cache");
                            }
                        }
                        ChainMessage::Reply(line, name, sender, names, replier) => {
                            replier.send(self.reply(&line, &name, &sender, &names)).expect("Could not send reply");
                        }
                        ChainMessage::NextNum(key, dir, sender) => {
                            let core = self.core.read().expect("Couldn't lock core for writing");
                            sender.send(if dir == 0 {
                                core.prev.get(&key).map_or(0, |list| list.len())
                            } else {
                                core.next.get(&key).map_or(0, |list| list.len())
                            })
                                .expect("Could not send next num");
                        }
                        ChainMessage::Next(key, dir, sender) => {
                            let mut core = self.core.write().expect("Couldn't lock core for writing");
                            sender.send(core.next(&key, dir)).expect("Could not send next word");
                        }
                        ChainMessage::Command(command, power, sender) => {
                            sender.send(self.command(&command, power)).expect("Could not send command response");
                        }
                        ChainMessage::RandomWord(sender) => {
                            let words = self.words.read().expect("Couldn't lock words for reading");
                            sender.send(words.get(random.next_u32() % words.len() as u32)).expect("Could not send random word");
                        }
                        ChainMessage::ChangeParent(parent) => {
                            self.set_parent(parent);
                        }
                        ChainMessage::Core(sender) => {
                            sender.send(self.core.clone()).expect("Couldn't send core");
                        }
                        ChainMessage::Stop => break,
                    }
                }
                println!("Unloading chain {}", self.filename);
            })
            .expect(&format!("Failed to spawn thread {}", filename));
        (sender, thread)
    }

    pub fn add_line(&mut self, line: &str, names: &[String]) -> Vec<(String, Vec<u32>)> {
        let mut learned = Vec::new();
        let line = &replace_names(line, names);
        for line in SENTENCE_END.split(line) {
            let words = self.words.clone();
            let mut words = words.write().expect("Failed to lock words for writing");
            let line_words: Vec<u32> = line.split(' ').map(|word| words.lookup(word.into())).collect();
            let mut hasher = self.build_hasher.build_hasher();
            line_words.hash(&mut hasher);
            let hash = hasher.finish();
            if self.tell_parent {
                if let Some(ref sender) = self.parent {
                    sender.send(ChainMessage::Learn(line.to_string(), Vec::new())).expect("Failed to make parent learn");
                }
            }
            if self.lines.insert(hash) {
                let mut core = self.core.write().expect("Couldn't lock core for writing");
                learned.push((line.to_string(), line_words.clone()));
                core.learn(line_words);
            }
            if line.contains(". ") {
                println!("Somehow missed {}", line)
            }
        }
        learned
    }

    pub fn reply(&mut self, input: &str, name: &str, sender: &str, names: &[String]) -> String {
        if input.is_empty() {
            return "".to_string();
        }
        let mut names = {
            let mut vec = Vec::new();
            vec.extend_from_slice(names);
            vec
        };
        if names.is_empty() {
            names.push(name.into());
            names.push(sender.into());
        }
        let input = replace_names(input, &names);

        let mut inputs: Vec<String> = SENTENCE_END.split(&input).map(|input| input.to_string()).collect();
        let input = inputs.pop().expect(&format!("No input lines {:?}, {:#?}", input, inputs));
        let mut replies = inputs.iter()
            .map(|input| self.reply(input, name, sender, &names))
            .filter(|reply| !reply.is_empty())
            .collect::<Vec<String>>();

        let input: Vec<u32> = {
            let mut words = self.words.write().expect("Failed to lock words for writing");
            input.split(' ').map(|word| words.lookup(word.into())).collect()
        };

        if !input.is_empty() {
            let random_sentence = "\0rand\0" != sender;
            let mut strict_limit = false;
            // ideal is the length of the sentence if it's not random, unlimited if name isn't a number
            let ideal = if random_sentence {
                input.len()
            } else if name.ends_with('!') {
                strict_limit = true;
                name[..name.len() - 1].parse().unwrap_or(0)
            } else {
                name.parse().unwrap_or(0)
            };

            let sentence = {
                let mut core = self.core.write().expect("Couldn't lock core for writing");
                // Pointless if it's generating a random sentence from a key
                let best = if random_sentence || input.len() > 3 {
                    core.choose_best(&input)
                } else {
                    [input[0], *input.get(1).unwrap_or(&NONE), *input.get(2).unwrap_or(&NONE)]
                };
                core.generate(best, ideal, strict_limit)
            };


            let mut name_replacements: HashMap<String, String> = HashMap::new();
            lazy_static! {
            static ref NAME_TOKEN: regex::Regex = regex::Regex::new(r"@name\d+@").expect(r"Failed to make regex @name\d+@");
        }

            let words = self.words.read().expect("Failed to lock words for reading");
            let mut random = OsRng::new().unwrap();
            let mut sentence = sentence.iter()
                .map(|&word| {
                    let word_map = &words.get_all(word);
                    let mut chances: Vec<_> = word_map.iter().map(|(word, count)| Weighted {weight: *count, item: word}).collect();
                    let chooser = WeightedChoice::new(&mut chances);
                    let word = chooser.ind_sample(&mut random);
                    NAME_TOKEN.replace_all(word, |caps: &regex::Captures| {
                        name_replacements.entry(caps[0].to_string())
                            .or_insert_with(|| names[random.next_u32() as usize % names.len()].clone())
                            .clone()
                    }).to_string()
                })
                .collect::<Vec<_>>()
                .join(" ");

            if !sentence.is_empty() && !URL_REGEX.is_match(&sentence) {
                let first = sentence.remove(0).to_uppercase().next().expect("How'd it get empty?");
                sentence.insert(0, first);
            }
            replies.push(sentence);
        }
        replies.join(". ")
    }

    pub fn random_sentence(&mut self, names: &[String], ideal: &str) -> String {
        let start = {
            let core = self.core.read().expect("Couldn't lock core for writing");
            let offset = OsRng::new().unwrap().next_u32() as usize % core.next.len();
            let empty_key = [END, END, END];
            let key = core.next.keys().nth(offset).unwrap_or(&empty_key);
            let mut sentence = vec![key[0], key[1], key[2]];
            sentence.retain(|&word| word < END);
            let words = self.words.read().expect("Failed to get words for reading");
            sentence.iter().map(|&word| words.get(word)).collect::<Vec<String>>().join(" ")
        };
        self.reply(&start, ideal, "\0rand\0", names)
    }

    pub fn command(&mut self, command: &str, power: Power) -> String {
        let parts: Vec<String> = command.split(' ').map(|part| part.to_lowercase()).collect();
        match parts.get(0).map(String::as_ref) {
            Some("stats") => {
                let words = self.words.read().expect("Failed to get words for reading");
                let core = self.core.read().expect("Couldn't lock core for writing");
                format!("Forwards keys: {}, Backwards keys: {}, words: {}, lines: {}",
                        core.next.len(),
                        core.prev.len(),
                        words.len(),
                        self.lines.len())
            }
            Some("wordstats") => {
                let keys = {
                    let mut prev = Vec::with_capacity(3);
                    let mut next = Vec::with_capacity(3);
                    let mut unknown = false;
                    {
                        let words = self.words.read().expect("Failed to get words for reading");
                        for i in 0..3 {
                            match parts.get(i + 1) {
                                Some(word) => {
                                    if let Some(word) = words.find(word) {
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
                let core = self.core.read().expect("Couldn't lock core for reading");
                let maps = [&core.prev, &core.next];
                for dir in 0..2 {
                    if let Some(list) = maps[dir].get(&keys[dir]) {
                        let mut counts = HashMap::new();
                        let mut list = list.clone();
                        list.retain(|&word| word < END);
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
                let words = self.words.read().expect("Failed to get words for reading");
                let mut key = keys[1].to_vec();
                key.retain(|&word| word < END);
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
                                    stats[1].count,
                                    words.get(stats[1].best),
                                    stats[1].best_count)
                        })
            }
            Some("sentence") => {
                self.random_sentence(&[CONFIG.name.clone(), CONFIG.owner.clone()],
                                     parts.get(1).unwrap_or(&"".to_string()))
            }
            Some("parent") => {
                match self.parent {
                    Some(ref parent) => {
                        let mut parts = parts.clone();
                        parts.remove(0);
                        let command = parts.join(" ");
                        let (sender, receiver) = channel();
                        parent.send(ChainMessage::Command(command, power, sender)).expect("Failed to send command to parent");
                        receiver.recv().expect("Failed to recieve answer from parent")
                    }
                    None => self.reply("No parent", "", "", &parts),
                }
            }
            Some("orphan") => {
                let mut parts = parts.clone();
                parts.remove(0);
                let command = parts.join(" ");
                let parent = self.parent.clone();
                self.set_parent(None);
                let result = self.command(&command, power);
                self.set_parent(parent);
                result
            }
            Some("strength") => {
                if parts.len() == 1 {
                    let core = self.core.read().expect("Couldn't lock core for writing");
                    core.strength.to_string()
                } else if power == Power::Cool {
                    if let Ok(strength) = parts[1].parse() {
                        self.set_strength(strength);
                        "".into()
                    } else {
                        self.reply(&parts.join(" "), "", "", &parts)
                    }
                } else {
                    self.reply("Not cool enough", "", "", &parts)
                }
            }
            Some("nsfw") => {
                if power == Power::Cool {
                    if parts.len() == 1 {
                        (!self.tell_parent).to_string()
                    } else if let Ok(tell_parent) = parts[1].parse::<bool>() {
                        self.tell_parent = !tell_parent;
                        "".to_string()
                    } else {
                        self.reply(&parts.join(" "), "", "", &parts)
                    }
                } else {
                    self.reply("Not cool enough", "", "", &parts)
                }
            }
            Some("filename") => {
                self.filename.clone()
            }
            Some("power") => {
                format!("{:?}", power)
            }
            _ => "".into(),
        }
    }

    pub fn set_parent(&mut self, parent: Option<Sender<ChainMessage>>) {
        self.parent = parent;
        self.core.write().expect("Couldn't lock core for writing").parent = match self.parent {
            Some(ref parent) => {
                let (sender, reciever) = channel();
                parent.send(ChainMessage::Core(sender.clone())).unwrap();
                Some(reciever.recv().unwrap())
            }
            None => None
        };
    }
    pub fn set_strength(&mut self, strength: f32) {
        let mut core = self.core.write().expect("Couldn't lock core for writing");
        core.strength = strength;
    }
}

impl WordMap {
    pub fn new() -> WordMap {
        Default::default()
    }

    pub fn lookup(&mut self, word: String) -> u32 {
        let mut real_word = String::new();
        for c in word.chars() {
            let letters = letters_only(&c.to_string());
            real_word += &if !letters.is_empty() {
                letters
            } else {
                c.to_lowercase().to_string()
            }
        }
        match self.ids.entry(real_word) {
            Entry::Occupied(entry) => {
                let index = *entry.get();
                *self.words[index as usize].entry(word).or_insert(0) += 1;
                index
            }
            Entry::Vacant(entry) => {
                self.words.push(ordermap!(word => 1));
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
        self.words[id as usize % self.words.len()].get_index(0).unwrap().0.clone()
    }
    
    pub fn get_all(&self, id: u32) -> OrderMap<String, u32> {
        self.words[id as usize % self.words.len()].clone()
    }

    pub fn len(&self) -> usize {
        self.words.len()
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Power {
    Normal,
    Cool
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
struct GlobalConfig {
    owner: String,
    name: String,
    chats: BTreeSet<String>,
}

impl GlobalConfig {
    pub fn load(filename: &str) -> GlobalConfig {
        let config = if let Ok(file) = File::open(filename) {
            serde_json::from_reader(file).expect("Failed to parse global config file")
        } else {
            GlobalConfig::default()
        };
        config.save(filename);
        config
    }

    pub fn save(&self, filename: &str) {
        serde_json::to_writer_pretty(&mut File::create(filename).expect("Could not create global config file"), &self).expect("Failed to write discord config file");
    }
}

impl Default for GlobalConfig {
    fn default() -> Self {
        let mut chats = BTreeSet::new();
        chats.insert("tumblr".to_string());
        chats.insert("twitter".to_string());
        chats.insert("discord".to_string());
        GlobalConfig {
            owner: "Mraof™".to_string(),
            name: "Simumech".to_string(),
            chats,
        }
    }
}

pub struct ChainCore {
    pub next: HashMap<Key, Vec<u32>>,
    pub prev: HashMap<Key, Vec<u32>>,
    pub random: OsRng,
    pub parent: Option<Arc<RwLock<ChainCore>>>,
    pub consume: f32,
    ///Lower it is, the higher the chance of using the parent
    pub strength: f32,
}

impl ChainCore {
    pub fn new() -> ChainCore {
        ChainCore {
            random: OsRng::new().unwrap(),
            next: Default::default(),
            prev: Default::default(),
            consume: 0.0,
            strength: 1.0,
            parent: None,
        }
    }
    pub fn learn(&mut self, mut keys: Vec<u32>) {
        keys.insert(0, END);
        keys.insert(0, END);
        keys.push(END);
        keys.push(END);
        let mut prev3;
        let mut prev2;
        let mut prev1;
        let mut next3;
        let mut next2;
        let mut next1;
        let end = keys.len() - 1;
        for (i, &word) in keys.iter().enumerate().take(end).skip(2) {
            if i > 2 {
                next3 = [keys[i - 3], keys[i - 2], keys[i - 1]];
                next2 = [next3[1], next3[2], NONE];
                next1 = [next3[2], NONE, NONE];
                self.next.entry(next3).or_insert_with(Default::default).push(word);
                self.next.entry(next2).or_insert_with(Default::default).push(word);
                self.next.entry(next1).or_insert_with(Default::default).push(word);
            }
            if i < end - 1 {
                prev3 = [keys[i + 2], keys[i + 1], word];
                prev2 = [prev3[1], prev3[2], NONE];
                prev1 = [prev3[2], NONE, NONE];
                self.prev.entry(prev3).or_insert_with(Default::default).push(keys[i - 1]);
                self.prev.entry(prev2).or_insert_with(Default::default).push(keys[i - 1]);
                self.prev.entry(prev1).or_insert_with(Default::default).push(keys[i - 1]);
            }
        }
    }

    fn generate(&mut self, best: Key, ideal: usize, strict_limit: bool) -> Vec<u32> {
        let mut sentence = vec![best[0], best[1], best[2]];
        sentence.retain(|&word| word < END);
        for dir in 0..2 {
            let mut words_temp: HashMap<Key, Vec<u32>> = HashMap::new();
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
                    match words_temp.get(&keys[1]).unwrap_or(&Vec::new()).len() {
                        0 => {
                            if (ideal as f32) / (sentence.len() as f32) < self.random.next_f32() {
                                break;
                            }
                            0
                        }
                        two_length => {
                            if !words_temp.get(&keys[2]).unwrap_or(&Vec::new()).is_empty() && self.random.next_f32() > 4.0 / (two_length as f32) {
                                2
                            } else {
                                1
                            }
                        }
                    }
                };
                if let Some(list) = words_temp.get_mut(&keys[key_index]) {
                    if !list.is_empty() {
                        let mut index = self.random.next_u32() as usize % list.len();
                        if ideal > 0 {
                            // Want a long sentence, but it's too much too long
                            if strict_limit && sentence.len() >= ideal / (2 - dir as usize) {
                                continue;
                            } else {
                                for _ in ideal..(sentence.len() + list.len() / 2) {
                                    index = self.random.next_u32() as usize % list.len();
                                    if let Some(word) = list.get(index) {
                                        if word == &END {
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        word = list.remove(index);
                    } else {
                        word = END
                    }
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
        sentence
    }

    fn choose_best(&self, input: &[u32]) -> Key {
        let mut best = [input[0], *input.get(1).unwrap_or(&NONE), *input.get(2).unwrap_or(&NONE)];
        let mut temp_keys = vec![[END, NONE, NONE], [END, END, NONE], [END, END, END]];
        let mut best_size = self.next.get(&best).map_or(0, |list| list.len());
        for word in input {
            temp_keys[2] = [temp_keys[2][1], temp_keys[2][2], *word];
            temp_keys[1] = [temp_keys[2][1], temp_keys[2][2], NONE];
            temp_keys[0] = [temp_keys[2][2], NONE, NONE];
            for key in &temp_keys {
                if key[0] != END && key[1] != END && key[2] != END {
                    let temp_size = self.next.get(key).map_or(0, |list| list.len());
                    if (temp_size > 0) && (best_size == 0 || (temp_size < best_size) || (temp_size == best_size && best[0] <= key[0])) {
                        best = *key;
                        best_size = temp_size;
                    }
                }
            }
        }
        best
    }

    pub fn next(&mut self, key: &Key, dir: u8) -> Option<Vec<u32>> {
        let option = if self.consume > self.random.next_f32() {
            if dir == 0 {
                self.prev.remove(key)
            } else {
                self.next.remove(key)
            }
        } else {
            match if dir == 0 {
                self.prev.get(key)
            } else {
                self.next.get(key)
            } {
                Some(list) => Some(list.clone()),
                None => None,
            }
        };
        match option {
            Some(list) => {
                match self.parent {
                    Some(ref parent) => {
                        let mut parent = parent.write().expect("Failed to get parent for writing");
                        let num = if dir == 0 {
                            parent.prev.get(key).map_or(0, |list| list.len())
                        } else {
                            parent.next.get(key).map_or(0, |list| list.len())
                        };
                        if num > 0 && self.random.next_u32() % num as u32 > self.random.next_u32() % ((list.len() * list.len()) as f32 * self.strength + (1.0 - self.strength)) as u32 {
                            parent.next(key, dir)
                        } else {
                            Some(list)
                        }
                    }
                    None => Some(list),
                }
            }
            None => {
                match self.parent {
                    Some(ref parent) => {
                        let mut parent = parent.write().expect("Failed to get parent for writing");
                        parent.next(key, dir)
                    }
                    None => None,
                }
            }
        }
    }
}

impl Default for ChainCore {
    fn default() -> Self {
        ChainCore::new()
    }
}

fn letters_only(input: &str) -> String {
    use unicode_normalization::UnicodeNormalization;

    input.nfkd()
        .map(|c| if c >= '\u{1f150}' && c <= '\u{1f169}' {
            unsafe { ::std::char::from_u32_unchecked(c as u32 - 0x1f150 + 'A' as u32) }
        } else if c >= '\u{1f170}' && c <= '\u{1f189}' {
            unsafe { ::std::char::from_u32_unchecked(c as u32 - 0x1f170 + 'A' as u32) }
        } else if c >= '\u{1f1e6}' && c <= '\u{1f1ff}' {
            unsafe { ::std::char::from_u32_unchecked(c as u32 - 0x1f1e6 + 'A' as u32) }
        } else {
            c
        }.to_ascii_lowercase())
        .filter(|c| c.is_lowercase() && c.is_ascii())
        .collect::<String>()
}

#[test]
fn test_commands() {
    let words = Arc::new(RwLock::new(WordMap::new()));
    let tumblr_chain = MarkovChain::new(words.clone(), "lines/tumblr");
    let sender = tumblr_chain.thread().0;
    let mut markov_chain = MarkovChain::new(words.clone(), "lines/twitter");
    markov_chain.set_parent(Some(sender));
    for _ in 0..100 {
        println!("{}", markov_chain.random_sentence(&vec!["Mraof".into(), "Grimble".into()], ""));
    }
}

#[test]
fn test_word_chain() {
    let mut chain = ChainCore::new();
    chain.consume = 0.1;
    let words: Vec<&str> = vec!["meow", "grow", "tree", "trombone", "Grombo", "grombu", "mrow", "meeooww"];
    for word in words {
        let keys: Vec<u32> = word.chars().map(|c| c as u32).collect();
        chain.learn(keys);
    }
    let mut random = OsRng::new().unwrap();
    for _ in 0..100 {
        let offset = random.next_u32() as usize % chain.next.len();
        let empty_key = [END, END, END];
        let key = *chain.next.keys().nth(offset).unwrap_or(&empty_key);
        let word: String = chain.generate(key, 0, false).into_iter().map(|c| unsafe { std::char::from_u32_unchecked(c) }).collect();
        println!("{}", word);
    }
}

