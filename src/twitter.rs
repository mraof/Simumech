use std::sync::mpsc::{channel, Sender, Receiver};
use std::sync::{RwLock, Arc};
use super::{WordMap, ChainMessage, MarkovChain};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::thread::Builder;
use std::thread::sleep;
use std::time::{Duration, UNIX_EPOCH};
use egg_mode;

pub fn start(main_chain: Sender<ChainMessage>, words: Arc<RwLock<WordMap>>) -> Sender<String> {
    let (sender, reciever): (_, Receiver<String>) = channel();
    let mut config = BufReader::new(File::open("config/twitter").unwrap()).lines();
    Builder::new()
        .name("twitter".to_string())
        .spawn(move || {
            let mut chain = MarkovChain::new(words, "lines/twitter");
            chain.parent = Some(main_chain);
            let chain = chain.thread().0;
            let (markov_sender, markov_reciever) = channel();

            let token = egg_mode::Token::new(config.next().unwrap().unwrap(), config.next().unwrap().unwrap());
            let access_token = egg_mode::Token::new(config.next().unwrap().unwrap(), config.next().unwrap().unwrap());
            loop {
                if let Ok(command) = reciever.try_recv() {
                    if let "stop" = command.as_ref() {
                        break
                    }
                }
                let seconds = UNIX_EPOCH.elapsed().unwrap().as_secs();
                sleep(Duration::from_secs((seconds * seconds) % 12000 + 30));
                chain.send(ChainMessage::Command("sentence 15".into(), markov_sender.clone())).expect("Failed to send command to chain");
                let mut tweet = markov_reciever.recv().expect("Failed to get reply");
                for _ in 0..10 {
                    if tweet.len() <= 140 {
                        break;
                    }
                    chain.send(ChainMessage::Command("sentence".into(), markov_sender.clone())).expect("Failed to send command to chain");
                    tweet = markov_reciever.recv().expect("Failed to get reply");
                }
                tweet.truncate(140);
                egg_mode::tweet::DraftTweet::new(&tweet).send(&token, &access_token).expect("Failed to post tweet");
            }
        }).unwrap();
    sender
}
