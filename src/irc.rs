use std::thread;
use std::collections::BTreeMap;
use irc_lib::client::prelude::*;
use std::sync::mpsc::{channel, Sender};

#[derive(Default)]
pub struct IRC {
    connections: BTreeMap<String, Sender<String>>,
}

impl IRC {
    pub fn init() -> IRC {
        let mut chat: IRC = Default::default();
        chat.connect(Config {
            nickname: Some("sbnkalnyBeta".to_string()),
            server: Some("localhost".to_string()),
            channels: Some(vec!["#test".to_string()]),
            ..Default::default()
        });

        chat
    }

    pub fn quit(&mut self) {}

    pub fn connect(&mut self, config: Config) {
        let nickname = config.nickname().to_string();
        let server = IrcServer::from_config(config).unwrap();
        server.identify().unwrap();
        let (sender, commands) = channel();
        self.connections.insert(nickname.clone(), sender);

        let server_clone = server.clone();
        let _ = thread::Builder::new().name(nickname.clone()).spawn(move || {
            let (message_sender, messages) = channel();
            thread::Builder::new().name("irciter".into()).spawn(move || for message in server_clone.iter() {
                message_sender.send(message).unwrap();
            });
            loop {
                while let Ok(message) = messages.try_recv() {
                    let message = message.unwrap();
                    println!("{:?}", message.clone());
                    match message.command {
                        Command::PRIVMSG(ref target, ref msg) => server.send_privmsg(target, msg).unwrap(),
                        _ => (),
                    };
                }
                while let Ok(command) = commands.try_recv() {
                    println!("{:?}", command);
                }
                thread::sleep_ms(50);
            }
        });
    }
}
