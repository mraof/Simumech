use discord_lib::{Discord, State, Error, ChannelRef};
use discord_lib::model::{Event, Game, OnlineStatus};
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
use std::time::{Instant, Duration, UNIX_EPOCH};

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
            let mut last_change = Instant::now();
            let mut change_time = Duration::from_secs(0);
            let (markov_sender, markov_reciever) = channel();
            loop {
                if let Ok(message) = reciever.try_recv() {
                    match message.to_lowercase().as_ref() {
                        "stop" => {
                            connection.shutdown().unwrap();
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
                connection.download_all_members(&mut state);
                if last_change.elapsed() > change_time {
                    last_change = Instant::now();
                    change_time = Duration::from_secs(UNIX_EPOCH.elapsed().unwrap().as_secs() & 1023);
                    let game_type = change_time.as_secs();
                    connection.set_game(if game_type > 0 {
                        main_chain.send(ChainMessage::RandomWord(markov_sender.clone())).unwrap();
                        let game = markov_reciever.recv().unwrap();
                        Some(if game_type == 1 {Game::playing(game)} else {Game::streaming(game.clone(), game)})
                    } else {
                        None
                    });
                    connection.sync_servers(&state.all_servers());
                }
                match event {
                    Event::MessageCreate(message) => {
                        let mut private = false;
                        if message.author.id != state.user().id && !message.author.bot {
                            connection.sync_calls(&[message.channel_id]);
                            let (mut replace_names, users) = match state.find_channel(&message.channel_id) {
                                Some(ChannelRef::Public(server, channel)) => {
                                    println!("[Discord] [{} #{}] {}: {}",
                                             server.name,
                                             channel.name,
                                             message.author.name,
                                             message.content);
                                    connection.sync_servers(&[server.id]);
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
                                    let mut users: Vec<String> = server.members.iter().map(|member| member.nick.clone().unwrap_or(member.user.name.clone()).clone()).collect();
                                    let mut presences = Vec::new();
                                    presences.push(message.author.name.clone());
                                    presences.push(state.user().username.clone());
                                    for presence in server.presences.iter() {
                                        let mut nick = presence.nick.clone();
                                        if let Some(nick) = nick.clone() {
                                            users.push(nick);
                                        } else if let Some(ref user) = presence.user {
                                            nick = Some(user.name.clone());
                                        }
                                        if let Some(nick) = nick {
                                            if presence.status == OnlineStatus::Online {
                                                presences.push(nick);
                                            }
                                        }
                                    }

                                    (users, presences)
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
                                    (users.clone(), users)
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
                                    private = true;
                                    (vec![name.clone(), message.author.name.clone()], vec![name.clone(), message.author.name.clone()])
                                }
                                None => {
                                    println!("[Discord] [Unknown Channel] {}: {}",
                                             message.author.name,
                                             message.content);
                                    channel_chains.entry(message.channel_id).or_insert_with(|| main_chain.clone());
                                    private = true;
                                    (vec![name.clone(), message.author.name.clone()], vec![name.clone(), message.author.name.clone()])
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
                            } else {
                                for user in message.mentions {
                                    replace_names.push(format!("{}", user.mention()));
                                }
                                if private || message.content.to_lowercase().contains(&name) {
                                    chain.send(ChainMessage::Reply(message.content.clone(),
                                                                   name.clone(),
                                                                   message.author.name.clone(),
                                                                   users.clone(),
                                                                   markov_sender.clone()))
                                        .unwrap();
                                    let response = markov_reciever.recv().unwrap();
                                    println!("Saying {}", response);
                                    let _ = discord.send_message(&message.channel_id,
                                                                 &response,
                                                                 "",
                                                                 false);
                                }
                                chain.send(ChainMessage::Learn(message.content, replace_names)).unwrap();
                            }
                        }
                    }
                    _ => (),
                }
            }
        })
        .unwrap();
    sender
}
