use discord_lib::{Discord, State, Error, ChannelRef};
use discord_lib::model::{Event, Game, OnlineStatus};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::sync::{RwLock, Arc};
use super::WordMap;
use super::ChainMessage;
use super::MarkovChain;
use super::Power;
use std::thread::Builder;
use std::collections::{HashMap, HashSet};
use retry::Retry;
use std::fs::File;
use std::io::{Read, Write};
use std::time::{Instant, Duration, UNIX_EPOCH};
use hyper::Client;
use super::regex::Regex;
use super::serde_json;

lazy_static! {
    static ref MENTION_REGEX: Regex = Regex::new(r"<@!?\d+>").expect("Failed to make MENTION_REGEX");
    static ref BRIDGE_REGEX: Regex = Regex::new(r"^[_\*~]*<(.*?)>[_\*~]* (.*)").expect("Failed to make BRIDGE_REGEX");
    static ref PIKAGIRL_PAY_REGEX: Regex = Regex::new(r"💰.*Transaction of (\d+) coin.*<@!?(\d+)> to <@!?(\d+)>.*").expect("Failed to make PIKAGIRL_PAY_REGEX");
}

pub fn start(main_chain: Sender<ChainMessage>, words: Arc<RwLock<WordMap>>) -> Sender<String> {
    let (sender, reciever): (_, Receiver<String>) = channel();
    let mut config = DiscordConfig::load("config/discord.json");
    Builder::new()
        .name("discord".to_string())
        .spawn(move || {
            let hyper_client = Arc::new(Client::new());
            //I have no idea how to get a user token normally
            #[allow(deprecated)]
            let discord = Discord::new_cache("config/discord_tokens", &config.login.email, Some(&config.login.password))
                .expect("Login failed");
            let (mut connection, ready) = Retry::new(&mut || discord.connect(), &mut |result| result.is_ok())
                .wait_between(200, 60000)
                .execute()
                .unwrap()
                .unwrap();
            let mut state = State::new(ready);
            let mut server_chains = HashMap::new();
            let mut channel_chains = HashMap::new();
            let mut user_chains = HashMap::new();
            let nsfw_words = Arc::new(RwLock::new(WordMap::new()));

            let name = state.user().username.to_lowercase();
            let mut last_change = Instant::now();
            let mut change_time = Duration::from_secs(0);
            let (markov_sender, markov_reciever) = channel();
            let mut name_map: HashMap<_, String> = HashMap::new();
            loop {
                if let Ok(message) = reciever.try_recv() {
                    if "stop" == message.to_lowercase() {
                        connection.shutdown().unwrap();
                        break;
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
                if let Event::MessageCreate(message) = event {
                    let mut private = false;
                    let mut listen = true;
                    let mut content = message.content.clone();
                    let mut author_name = message.author.name.clone();
                    //Bot interactions
                    if message.author.bot {
                        listen = false;
                        match message.author.id.0 {
                            //PikaGirl
                            169678500893163520 => {
                                println!("Gambling! {:?}", message.mentions);
                                if let Some(cap) = PIKAGIRL_PAY_REGEX.captures(&message.content) {
                                    let amount = cap.get(1).expect("No amount captured");
                                    let payer = cap.get(2).expect("No payer captured");
                                    let paid = cap.get(3).expect("No paid captured");
                                    if paid.as_str().parse::<u64>().expect("User id wasn't a number") == state.user().id.0 {
                                        println!("{} paid {} to me~", payer.as_str(), amount.as_str());
                                    }
                                }
                            },
                            229824641160577025 => {
                                if let Some(cap) = BRIDGE_REGEX.captures(&message.content) {
                                    author_name = cap.get(1).expect("No author captured").as_str().to_string();
                                    content = cap.get(2).expect("No content captured").as_str().to_string();
                                    listen = true;
                                }
                            },
                            _ => {
                                println!("Unhandled bot {}", author_name);
                            }
                        }
                    }
                    listen = listen && !config.ignored.contains(&message.channel_id.0);

                    let power = if message.author.id.to_string() == config.owner_id {
                        Power::Cool
                    } else {
                        Power::Normal
                    };
                    let mut nsfw = false;
                    if message.author.id != state.user().id {
                        connection.sync_calls(&[message.channel_id]);
                        let (mut replace_names, users) = match state.find_channel(message.channel_id) {
                            Some(ChannelRef::Public(server, channel)) => {
                                println!("[Discord] [{} #{}] {}: {}",
                                         server.name,
                                         channel.name,
                                         author_name,
                                         content);
                                nsfw = channel.name.contains("nsfw");
                                if !nsfw {
                                    for attachment in message.attachments.clone() {
                                        //Having dimensions means it's an image
                                        if attachment.dimensions.is_some() {
                                            let filename = format!("images/discord/{}/{}.{}", server.id, attachment.id, attachment.filename.split(".").last().unwrap());
                                            let client = hyper_client.clone();
                                            Builder::new()
                                                .name(filename.clone())
                                                .spawn(move || {
                                                    println!("[Discord] Downloading {} to {}", attachment.filename, &filename);
                                                    if let Ok(response) = client.get(&attachment.proxy_url).send() {
                                                        let mut file = File::create(&filename).expect("Failed to create file");
                                                        file.write_all(&response.bytes().map(|byte| byte.unwrap()).collect::<Vec<u8>>()).expect("Failed to write to file");
                                                    }
                                                }).expect("Failed to spawn thread");
                                        }
                                    }
                                }
                                connection.sync_servers(&[server.id]);
                                channel_chains.entry(channel.id).or_insert_with(|| {
                                    let mut chain = MarkovChain::new(if nsfw {
                                        println!("{} is using nsfw_words", server.id);
                                        nsfw_words.clone()
                                    } else {
                                        words.clone()
                                    },
                                 &format!("lines/discord/{}/{}", server.id, channel.id));
                                    chain.parent = Some(server_chains.entry(server.id)
                                        .or_insert_with(|| {
                                            let mut chain = MarkovChain::new(words.clone(),
                                                                             &format!("lines/discord/{}/server", server.id));
                                            chain.parent = Some(main_chain.clone());
                                            chain.thread().0
                                        })
                                        .clone());
                                    if nsfw {
                                        chain.tell_parent = false
                                    }
                                    chain.strength = 0.7;
                                    chain.thread().0
                                });
                                let mut users: Vec<String> = server.members.iter().map(|member| member.nick.clone().unwrap_or_else(|| member.user.name.clone()).clone()).collect();
                                let mut presences = Vec::new();
                                presences.push(author_name.clone());
                                presences.push(state.user().username.clone());
                                for presence in &server.presences {
                                    let mut nick = presence.nick.clone();
                                    if let Some(nick) = nick.clone() {
                                        users.push(nick);
                                    } else if let Some(ref user) = presence.user {
                                        nick = Some(user.name.clone());
                                    } else if let Some(name) = name_map.get(&presence.user_id) {
                                        nick = Some(name.clone());
                                    }
                                    if let Some(nick) = nick {
                                        name_map.insert(presence.user_id, nick.clone());
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
                                         author_name,
                                         content);
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
                                         author_name,
                                         content);
                                channel_chains.entry(channel.id).or_insert_with(|| {
                                    let mut chain = MarkovChain::new(words.clone(),
                                                                     &format!("lines/discord/{}/server", channel.id));
                                    chain.parent = Some(main_chain.clone());
                                    chain.tell_parent = false;
                                    chain.thread().0
                                });
                                private = true;
                                (vec![name.clone(), author_name.clone()], vec![name.clone(), author_name.clone()])
                            }
                            None => {
                                println!("[Discord] [Unknown Channel] {}: {}",
                                         author_name,
                                         content);
                                channel_chains.entry(message.channel_id).or_insert_with(|| main_chain.clone());
                                private = true;
                                (vec![name.clone(), author_name.clone()], vec![name.clone(), author_name.clone()])
                            }
                        };
                        let mut chain = &channel_chains[&message.channel_id];
                        if !(private || nsfw) {
                            let user_chain = user_chains.entry(message.author.id).or_insert_with(|| {
                                let mut chain = MarkovChain::new(words.clone(),
                                                                 &format!("lines/discord/users/{}", message.author.id));
                                chain.strength = 0.3;
                                chain.thread().0
                            });
                            user_chain.send(ChainMessage::ChangeParent(Some(chain.clone()))).expect("Couldn't change parent");
                            chain = user_chain;
                        }
                        if listen {
                            if content.starts_with("$m") && content.len() >= 3 {
                                let mut command = content.clone();
                                command.drain(..3);
                                main_chain.send(ChainMessage::Command(command, power, markov_sender.clone())).expect("Couldn't send Command to chain");
                                let _ = discord.send_message(message.channel_id,
                                                             &markov_reciever.recv().unwrap(),
                                                             "",
                                                             false);
                            } else if content.starts_with("$cm") && content.len() >= 4 {
                                let mut command = content.clone();
                                command.drain(..4);
                                chain.send(ChainMessage::Command(command, power, markov_sender.clone())).unwrap();
                                let _ = discord.send_message(message.channel_id,
                                                             &markov_reciever.recv().unwrap(),
                                                             "",
                                                             false);
                            } else if content == "$ignore" && power == Power::Cool {
                                config.ignored.insert(message.channel_id.0);
                                config.save("config/discord.json");
                            } else {
                                for user in message.mentions {
                                    replace_names.push(format!("{}", user.mention()));
                                }
                                replace_names.push("@everyone".to_string());
                                replace_names.push("@here".to_string());
                                if private || content.to_lowercase().contains(&name) || weird_contains(&content, &name) {
                                    chain.send(ChainMessage::Reply(content.clone(),
                                                                   name.clone(),
                                                                   author_name.clone(),
                                                                   users.clone(),
                                                                   markov_sender.clone()))
                                        .expect("Couldn't send Reply to chain");
                                    let response = markov_reciever.recv().unwrap();
                                    let response = MENTION_REGEX.replace_all(&response, &format!("<@!{}>", config.owner_id)[..]);
                                    println!("Saying {}", response);
                                    let _ = discord.send_message(message.channel_id,
                                                                 &response,
                                                                 "",
                                                                 false);
                                }
                                //Nothing good ever comes from lines containing "daddy"
                                //TODO make this configurable
                                if !content.to_lowercase().contains("daddy") {
                                    chain.send(ChainMessage::Learn(content, replace_names)).expect("Couldn't send Learn to chain");
                                }
                            }
                        }
                    }
                }
            }
        })
        .unwrap();
    sender
}

fn weird_contains(message: &str, string: &str) -> bool {
    let message = message.split_whitespace().into_iter().collect::<String>();
    if message.len() < string.len() || string.is_empty() {
        return false;
    }
    let mut diffs: Vec<i32> = Vec::new();
    let mut chars = string.chars();
    let mut last = chars.next().unwrap();
    while let Some(c) = chars.next() {
        diffs.push(last as i32 - (c as i32));
        last = c;
    }
    let mut message_diffs: Vec<i32> = Vec::new();
    chars = message.chars();
    last = chars.next().unwrap();
    for c in chars {
        message_diffs.push(last as i32 - (c as i32));
        last = c;
    }
    if message_diffs.len() >= diffs.len() {
        for i in 0..(message_diffs.len() - diffs.len() + 1) {
            let mut matches = true;
            for (j, &diff) in diffs.iter().enumerate() {
                matches = message_diffs[i + j] == diff;
                if !matches {
                    break;
                }
            }
            if matches {
                return true;
            }
        }
    }
    false
}

#[derive(Default, Debug, Deserialize, Serialize)]
#[serde(default)]
struct DiscordConfig {
    login: DiscordLogin,
    owner_id: String,
    ignored: HashSet<u64>
}

#[derive(Default, Debug, Deserialize, Serialize)]
#[serde(default)]
struct DiscordLogin {
    email: String,
    password: String
}

impl DiscordConfig {
    pub fn load(filename: &str) -> DiscordConfig {
        let config = if let Ok(file) = File::open(filename) {
            serde_json::from_reader(file).expect("Failed to parse discord config file")
        } else {
            DiscordConfig::default()
        };
        config.save(filename);
        config
    }

    pub fn save(&self, filename: &str) {
        serde_json::to_writer_pretty(&mut File::create(filename).expect("Could not create discord config file"), &self).expect("Failed to write discord config file");
    }
}