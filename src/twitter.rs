use std::sync::mpsc::{channel, Sender, Receiver};
use std::sync::{RwLock, Arc};
use super::{WordMap, ChainMessage, MarkovChain, Power};
use std::fs::File;
use std::thread::Builder;
use std::thread::sleep;
use std::time::{Duration, UNIX_EPOCH};
use serde_json;
use egg_mode;
use tokio_core::reactor::Core;
use astro::lunar::Phase;

pub fn start(main_chain: Sender<ChainMessage>, words: Arc<RwLock<WordMap>>) -> Sender<String> {
    let (sender, reciever): (_, Receiver<String>) = channel();
    let config = TwitterConfig::load("config/twitter.json");
    let cli_sender = sender.clone();
    Builder::new()
        .name("twitter".to_string())
        .spawn(move || {
            let mut chain = MarkovChain::new(words, "lines/twitter");
            chain.parent = Some(main_chain);
            chain.set_strength(0.6);
            let chain = chain.thread().0;
            let (markov_sender, markov_reciever) = channel();

            let tweet_sender = sender.clone();
            Builder::new()
                .name("twitter_tweeter".to_string())
                .spawn(move || {
                    loop {
                        let seconds = UNIX_EPOCH.elapsed().unwrap().as_secs();
                        sleep(Duration::from_secs((seconds * seconds) % 12000 + 30));
                        tweet_sender.send("tweet".to_string()).expect("Failed to send answer command to tumblr reciever");
                    }
                }).expect("Unable to create twitter tweeter thread");

            let mut core = Core::new().unwrap();
            let handle = core.handle();
            
            let consumer_token = egg_mode::KeyPair::new(config.consumer_key, config.consumer_secret);
            let access_token = egg_mode::KeyPair::new(config.access_key, config.access_secret);
            let token = egg_mode::Token::Access { consumer: consumer_token, access: access_token };
            while let Ok(command) = reciever.recv() {
                chain.send(ChainMessage::Command(format!("strength {}", 0.6 * (super::astronomy::time_from_moon_phase(Phase::Full) / 15.0) as f32), Power::Cool, markov_sender.clone())).expect("Failed to send command to chain");
                markov_reciever.recv().expect("Failed to get reply");
                match command.as_ref() {
                    "stop" => {
                        break
                    }
                    "tweet" => {
                        chain.send(ChainMessage::Command("sentence 15".into(), Power::Normal, markov_sender.clone())).expect("Failed to send command to chain");
                        let mut tweet = markov_reciever.recv().expect("Failed to get reply");
                        for _ in 0..10 {
                            if tweet.len() <= 140 {
                                break;
                            }
                            chain.send(ChainMessage::Command("sentence 15".into(), Power::Normal, markov_sender.clone())).expect("Failed to send command to chain");
                            tweet = markov_reciever.recv().expect("Failed to get reply");
                        }
                        tweet.truncate(140);
                        if let Err(error) = core.run(egg_mode::tweet::DraftTweet::new(tweet.as_str()).send(&token, &handle)) {
                            println!("Tweet {} failed to send with error {:#?}", tweet, error);
                        }
                    }
                    _ => ()
                }
            }
        }).unwrap();
    cli_sender
}

#[derive(Default, Debug, Deserialize, Serialize)]
#[serde(default)]
struct TwitterConfig {
    consumer_key: String,
    consumer_secret: String,
    access_key: String,
    access_secret: String,
}

impl TwitterConfig {
    pub fn load(filename: &str) -> TwitterConfig {
        let config = if let Ok(file) = File::open(filename) {
            serde_json::from_reader(file).expect("Failed to parse twitter config file")
        } else {
            TwitterConfig::default()
        };
        config.save(filename);
        config
    }

    pub fn save(&self, filename: &str) {
        serde_json::to_writer_pretty(&mut File::create(filename).expect("Could not create twitter config file"), &self).expect("Failed to write discord config file");
    }
}
