use discord_lib::{Discord, State, Error, ChannelRef};
use discord_lib::model::Event;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::sync::{RwLock, Arc};
use super::WordMap;
use super::ChainMessage;
use super::MarkovChain;
use std::thread::Builder;
use std::collections::HashMap;
use retry::Retry;
use std::fs::File;
use std::io::{BufRead, BufReader};

pub fn start(main_chain: Sender<ChainMessage>, words: Arc<RwLock<WordMap>>) -> Sender<String> {
    let (sender, reciever): (_, Receiver<String>) = channel();
    let mut config = BufReader::new(File::open("config/discord").unwrap()).lines();
    Builder::new()
        .name("discord".to_string())
        .spawn(move || {
            let discord = Discord::new_cache("config/discord_tokens", &config.next().unwrap().unwrap(), Some(&config.next().unwrap().unwrap()))
                .expect("Login failed");
            let (mut connection, ready) = Retry::new(&mut || discord.connect(), &mut |result| result.is_ok())
                .wait_between(200, 60000)
                .execute()
                .unwrap()
                .unwrap();
            let mut state = State::new(ready);
            let mut server_chains = HashMap::new();
            let mut channel_chains = HashMap::new();
            let name = state.user().username.to_lowercase();
/*            for server in state.servers() {
                println!("server {:?}", server);
*//*                for channel in &server.channels {
                    println!("channel {:?}", channel)
                }*//*
            }*/
            let (markov_sender, markov_reciever) = channel();
            loop {
                if let Ok(message) = reciever.try_recv() {
                    match message.to_lowercase().as_ref() {
                        "stop" => {
                            break;
                        }
                        _ => (),
                    }
                }
                let event = match connection.recv_event() {
                    Ok(event) => event,
                    Err(err) => {
                        println!("[Warning] Receive error: {:?}", err);
                        if let Error::WebSocket(..) = err {
                            let (new_connection, ready) = Retry::new(&mut || discord.connect(), &mut |result| result.is_ok())
                                .wait_between(200, 60000)
                                .execute()
                                .unwrap()
                                .unwrap();
                            connection = new_connection;
                            state = State::new(ready);
                        }
                        continue;
                    }
                };
                state.update(&event);
                match event {
                    Event::MessageCreate(message) => {
                        if message.author.id != state.user().id && !message.author.bot {
                            let (replace_names, private) = match state.find_channel(&message.channel_id) {
                                Some(ChannelRef::Public(server, channel)) => {
                                    println!("[Discord] [{} #{}] {}: {}",
                                             server.name,
                                             channel.name,
                                             message.author.name,
                                             message.content);
                                    channel_chains.entry(channel.id).or_insert_with(|| {
                                        let mut chain = MarkovChain::new(words.clone(),
                                                                         &format!("lines/discord/{}/{}", server.id, channel.id));
                                        chain.parent = Some(server_chains.entry(server.id)
                                            .or_insert_with(|| {
                                                let mut chain = MarkovChain::new(words.clone(),
                                                                                 &format!("lines/discord/{}/server", server.id));
                                                chain.parent = Some(main_chain.clone());
                                                chain.thread().0
                                            })
                                            .clone());
                                        if channel.name.contains("nsfw") {
                                            chain.tell_parent = false
                                        }
                                        chain.thread().0
                                    });
                                    let users: Vec<String> = server.members.iter().map(|member| member.nick.clone().unwrap_or(member.user.name.clone()).clone()).collect();
                                    (users, false)
/*                                     server.presences
                                        .iter()
                                        .map(|presence| {
                                            presence.nick
                                                .clone()
                                                .unwrap_or(match presence.user {
                                                    Some(ref user) => user.name.clone(),
                                                    None => {
                                                        println!("No name found for {}", presence.user_id);
                                                        format!("{}", presence.user_id)},
                                                })
                                                .clone()
                                        })
                                        .collect())*/
                                }
                                Some(ChannelRef::Group(group)) => {
                                    println!("[Discord] [Group {}] {}: {}",
                                             group.name(),
                                             message.author.name,
                                             message.content);
                                    channel_chains.entry(group.channel_id).or_insert_with(|| {
                                        let mut chain = MarkovChain::new(words.clone(),
                                                                         &format!("lines/discord/{}", group.channel_id));
                                        chain.parent = Some(main_chain.clone());
                                        chain.thread().0
                                    });
                                    let users: Vec<String> = group.recipients.iter().map(|user| user.name.clone()).collect();
                                    (users, false)
                                }
                                Some(ChannelRef::Private(channel)) => {
                                    println!("[Discord] [Private] {}: {}",
                                             message.author.name,
                                             message.content);
                                    channel_chains.entry(channel.id).or_insert_with(|| {
                                        let mut chain = MarkovChain::new(words.clone(),
                                                                         &format!("lines/discord/{}/server", channel.id));
                                        chain.parent = Some(main_chain.clone());
                                        chain.thread().0
                                    });
                                    (vec![name.clone(), message.author.name.clone()], true)
                                }
                                None => {
                                    println!("[Discord] [Unknown Channel] {}: {}",
                                             message.author.name,
                                             message.content);
                                    channel_chains.entry(message.channel_id).or_insert_with(|| main_chain.clone());
                                    (vec![name.clone(), message.author.name.clone()], false)
                                }
                            };
                            let chain = channel_chains.get(&message.channel_id).unwrap();
                            if message.content.starts_with("$m") {
                                let mut command = message.content.clone();
                                command.drain(..2);
                                main_chain.send(ChainMessage::Command(command, markov_sender.clone())).unwrap();
                                let _ = discord.send_message(&message.channel_id,
                                                             &markov_reciever.recv().unwrap(),
                                                             "",
                                                             false);
                            } else if message.content.starts_with("$cm") {
                                let mut command = message.content.clone();
                                command.drain(..3);
                                chain.send(ChainMessage::Command(command, markov_sender.clone())).unwrap();
                                let _ = discord.send_message(&message.channel_id,
                                                             &markov_reciever.recv().unwrap(),
                                                             "",
                                                             false);
                            } else if private || message.content.to_lowercase().contains(&name) {
                                chain.send(ChainMessage::Reply(message.content.clone(),
                                                              name.clone(),
                                                              message.author.name.clone(),
                                                              replace_names.clone(),
                                                              markov_sender.clone()))
                                    .unwrap();
                                let response = markov_reciever.recv().unwrap();
                                println!("{:?}, {}", replace_names, response);
                                let _ = discord.send_message(&message.channel_id,
                                                  &response,
                                                  "",
                                                  false);
                            }
                            chain.send(ChainMessage::Learn(message.content, replace_names)).unwrap();
                        }
                    }
                    _ => (),
                }
            }
        })
        .unwrap();
    sender
}
