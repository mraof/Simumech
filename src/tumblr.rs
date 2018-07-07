use std::sync::mpsc::{channel, Sender, Receiver};
use std::sync::{RwLock, Arc};
use super::{WordMap, ChainMessage, MarkovChain, Power, ChainCore};
use std::fs::File;
use std::thread::Builder;
use std::thread::sleep;
use std::time::{Duration, UNIX_EPOCH};
use serde_json;
use tumblr_lib::{Tumblr, Response, ParamList, PostType};
use rand::OsRng;
use rand::RngCore;
use rand::Rng;
use rand::distributions::{Distribution, Weighted, WeightedChoice};
use std::char;
use astro::lunar::Phase;

pub fn start(main_chain: Sender<ChainMessage>, words: Arc<RwLock<WordMap>>) -> Sender<String> {
    let (sender, reciever): (_, Receiver<String>) = channel();
    let config_file = "config/tumblr.json";
    let mut config = TumblrConfig::load(config_file);
    let cli_sender = sender.clone();
    Builder::new()
        .name("tumblr".to_string())
        .spawn(move || {
            if config.blog.is_empty() {
                panic!("Blog can't be empty");
            }
            if config.askers.is_empty() {
                config.askers.push(config.blog.clone());
                config.askers.push(config.blog.clone());
                config.askers.push(config.blog.clone());
                config.askers.push(config.blog.clone());
                config.save(config_file);
            }
            let mut name_chain = ChainCore::new();
            for word in &config.askers {
                let keys: Vec<u32> = word.chars().map(|c| c as u32).collect();
                name_chain.learn(keys);
            }
            let mut chain = MarkovChain::new(words, "lines/tumblr");
            chain.parent = Some(main_chain.clone());
            chain.set_strength(config.strength);
            let chain = chain.thread().0;
            let (markov_sender, markov_reciever) = channel();
            for _ in 0..config.askers.len() {
                chain.send(ChainMessage::RandomWord(markov_sender.clone())).unwrap();
                let word = markov_reciever.recv().unwrap();
                let keys: Vec<u32> = word.chars().map(|c| c as u32).collect();
                name_chain.learn(keys);
            }
            let mut tumblr = Tumblr::new(&config.consumer_key, &config.consumer_secret);
            tumblr.set_token(&config.access_key, &config.access_secret);
            /*let answer_sender = sender.clone();
            Builder::new()
                .name("tumblr_answers".to_string())
                .spawn(move || {
                    loop {
                        answer_sender.send("answer".to_string()).expect("Failed to send answer command to tumblr reciever");
                        sleep(Duration::from_secs(600));
                    }
                }).expect("Unable to create tumblr answers thread");*/
            let post_sender = sender.clone();
            Builder::new()
                .name("tumblr_poster".to_string())
                .spawn(move || {
                    loop {
                        let seconds = UNIX_EPOCH.elapsed().unwrap().as_secs();
                        sleep(Duration::from_secs((seconds * seconds) % 18000 + 100));
                        post_sender.send("post".to_string()).expect("Failed to send answer command to tumblr reciever");
                    }
                }).expect("Unable to create tumblr poster thread");
            let mut post_chances = vec![
                Weighted {weight: 5, item: "TEXT"},
                Weighted {weight: 1, item: "CHAT"},
                Weighted {weight: 3, item: "QUOTE"},
            ];
            let post_chooser = WeightedChoice::new(&mut post_chances);
            let mut rng = OsRng::new().unwrap();
            while let Ok(command) = reciever.recv() {
                chain.send(ChainMessage::Command(format!("strength {}", config.strength * (super::astronomy::time_from_moon_phase(Phase::Full) / 15.0) as f32), Power::Cool, markov_sender.clone())).expect("Failed to send command to chain");
                markov_reciever.recv().expect("Failed to get reply");
                match command.as_ref() {
                    "stop" => {
                        break
                    }
                    "answer" => {
                        println!("Checking tumblr {} for asks", config.blog);
                        let mut params = ParamList::new();
                        params.insert("type".into(), "answer".into());
                        match tumblr.get::<Response>(&format!("/blog/{}/posts/submission", config.blog), Some(&params)) {
                            Ok(posts) => {
                                let posts = posts.posts.expect("Invalid response for submissions");
                                println!("Found {} asks", posts.len());
                                for post in posts {
                                    let post = post.into_post();
                                    if let PostType::Answer { answer, asking_name, asking_url, question, .. } = post.post_type {
                                        if !answer.is_empty() {
                                            println!("Tumblr ask already had an answer, {}", answer);
                                        }
                                        let mut tags = "ANSWER,".to_string();
                                        if asking_url.is_some() {
                                            tags += &asking_name;
                                            tags += ",";
                                            config.askers.push(asking_name.clone());
                                            config.save(config_file);

                                            let keys: Vec<u32> = asking_name.chars().map(|c| c as u32).collect();
                                            name_chain.learn(keys);
                                        }

                                        chain.send(ChainMessage::Reply(question.clone(), config.blog.clone(), asking_name.clone(), config.askers.clone(), markov_sender.clone())).expect("Failed to send command to chain");
                                        let answer = markov_reciever.recv().expect("Failed to get reply");

                                        chain.send(ChainMessage::Reply(question.clone(), config.blog.clone(), asking_name.clone(), config.askers.clone(), markov_sender.clone())).expect("Failed to send command to chain");
                                        tags += &markov_reciever.recv().expect("Failed to get reply");
                                        chain.send(ChainMessage::Learn(question.clone(), config.askers.clone())).expect("Couldn't learn ask");

                                        let mut params = ParamList::new();
                                        params.insert("id".into(), post.id.to_string().into());
                                        params.insert("state".into(), "queue".into());
                                        params.insert("answer".into(), answer.into());
                                        params.insert("tags".into(), tags.into());
                                        let _ = tumblr.post(&format!("/blog/{}/post/edit", config.blog), Some(&params)).map_err(|err| println!("{:?}", err));
                                    }
                                }
                            }
                            Err(error) => {
                                println!("{:#?}", error);
                            }
                        }
                    }
                    "post" => {
                        let post_type = post_chooser.sample(&mut rng);
                        let mut params = ParamList::new();
                        let mut tags = String::new() + post_type + ",";
                        println!("Making {} post", post_type);
                        match post_type {
                            "TEXT" => {
                                //Text
                                params.insert("type".into(), "text".into());
                                params.insert("native_inline_images".into(), "true".into());
                                let body = if rng.next_u32() % 20 == 0 {
                                    chain.send(ChainMessage::Command("sentence 5".into(), Power::Normal, markov_sender.clone())).expect("Failed to send command to chain");
                                    let mut title = markov_reciever.recv().expect("Failed to get reply");
                                    for i in 1..11 {
                                        if title.len() <= i * 10 {
                                            break;
                                        }
                                        main_chain.send(ChainMessage::Command("sentence 5".into(), Power::Normal, markov_sender.clone())).expect("Failed to send command to chain");
                                        title = markov_reciever.recv().expect("Failed to get reply");
                                    }
                                    params.insert("title".into(), title.clone().into());
                                    chain.send(ChainMessage::Reply(title.clone(), config.blog.clone(), config.blog.clone(), config.askers.clone(), markov_sender.clone())).expect("Failed to send command to chain");
                                    markov_reciever.recv().expect("Failed to get reply")
                                } else {
                                    chain.send(ChainMessage::Command("sentence".into(), Power::Normal, markov_sender.clone())).expect("Failed to send command to chain");
                                    markov_reciever.recv().expect("Failed to get reply")
                                };
                                params.insert("body".into(), body.clone().into());
                                chain.send(ChainMessage::Reply(body.clone(), config.blog.clone(), config.blog.clone(), config.askers.clone(), markov_sender.clone())).expect("Failed to send command to chain");
                                tags += &markov_reciever.recv().expect("Failed to get reply");
                            }
                            "CHAT" => {
                                //Chat
                                params.insert("type".into(), "chat".into());
                                let count = rng.next_u32() % 4 + 1;
                                let mut participants = Vec::with_capacity(count as usize);
                                for _ in 0..count {
                                    participants.push(rng.choose(&config.askers).unwrap().clone());
                                }
                                let mut conversation = String::new();
                                let mut last: String;
                                let mut last_person;
                                if rng.next_u32() % 20 == 0 {
                                    last_person = config.blog.clone();
                                    main_chain.send(ChainMessage::Command("sentence 5".into(), Power::Normal, markov_sender.clone())).expect("Failed to send command to chain");
                                    last = markov_reciever.recv().expect("Failed to get reply");
                                    params.insert("title".into(), last.clone().into());
                                } else {
                                    let current_person = rng.choose(&participants).unwrap().clone();
                                    main_chain.send(ChainMessage::Command("sentence".into(), Power::Normal, markov_sender.clone())).expect("Failed to send command to chain");
                                    last = markov_reciever.recv().expect("Failed to get reply");
                                    conversation += &format!("{}: {}\n", current_person, last);
                                    last_person = current_person;
                                }
                                for _ in 0..(rng.next_u32() % 6 + 1) * count {
                                    let current_person = rng.choose(&participants).unwrap().clone();
                                    chain.send(ChainMessage::Reply(last.clone(), current_person.clone(), last_person, config.askers.clone(), markov_sender.clone())).expect("Failed to send command to chain");
                                    last = markov_reciever.recv().expect("Failed to get reply");
                                    last = last.split(". ").last().unwrap().to_string();
                                    conversation += &format!("{}: {}\n", current_person, last);
                                    last_person = current_person;
                                }
                                params.insert("conversation".into(), conversation.into());
                                tags += &participants.join(",");
                            }
                            "QUOTE" => {
                                //Quote
                                params.insert("type".into(), "quote".into());
                                chain.send(ChainMessage::Command("sentence".into(), Power::Normal, markov_sender.clone())).expect("Failed to send command to chain");
                                let quote = markov_reciever.recv().expect("Failed to get reply");
                                params.insert("quote".into(), quote.clone().into());
                                if rng.next_u32() % 2 == 0 {
                                    let base_name = rng.choose(&config.askers).unwrap();
                                    let key = name_chain.choose_best(&base_name.chars().map(|c| c as u32).collect::<Vec<u32>>());
                                    let name: String = name_chain.generate(key, 5, false).into_iter().map(|c| unsafe {char::from_u32_unchecked(c)}).collect();
                                    params.insert("source".into(), name.into());
                                }
                                chain.send(ChainMessage::Reply(quote.clone(), config.blog.clone(), config.blog.clone(), config.askers.clone(), markov_sender.clone())).expect("Failed to send command to chain");
                                tags += &markov_reciever.recv().expect("Failed to get reply");
                            }
                            _ => ()
                        }
                        params.insert("tags".into(), tags.into());
                        println!("{:#?}", params);
                        let _ = tumblr.post(&format!("/blog/{}/post", config.blog), Some(&params)).map_err(|err| println!("{:?}", err));
                    }
                    "reload" => {
                        let new_config = TumblrConfig::load(config_file);
                        config.blog = new_config.blog;
                        config.strength = new_config.strength;
                        config.askers = new_config.askers;
                        chain.send(ChainMessage::Command(format!("strength {}", config.strength), Power::Cool, markov_sender.clone())).expect("Failed to send command to chain");
                        println!("{}", markov_reciever.recv().expect("Failed to get reply"));
                    }
                    _ => ()
                }
            }
        }).unwrap();
    cli_sender
}

#[derive(Default, Debug, Deserialize, Serialize)]
#[serde(default)]
struct TumblrConfig {
    consumer_key: String,
    consumer_secret: String,
    access_key: String,
    access_secret: String,
    blog: String,
    #[serde(default = "default_strength")]
    strength: f32,
    askers: Vec<String>,
}

impl TumblrConfig {
    pub fn load(filename: &str) -> TumblrConfig {
        let config = if let Ok(file) = File::open(filename) {
            serde_json::from_reader(file).expect("Failed to parse tumblr config file")
        } else {
            TumblrConfig::default()
        };
        config.save(filename);
        config
    }

    pub fn save(&self, filename: &str) {
        serde_json::to_writer_pretty(&mut File::create(filename).expect("Could not create tumblr config file"), &self).expect("Failed to write discord config file");
    }
}

fn default_strength() -> f32 {
    0.6
}
