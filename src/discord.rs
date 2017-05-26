use discord_lib::{Discord, State, Error, ChannelRef, Connection};
use discord_lib::model::{Event, Message, Game, OnlineStatus, ServerId, ChannelId, UserId};
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
    static ref MENTION_REGEX: Regex = Regex::new(r"@here|@everyone|<@!?\d+>").expect("Failed to make MENTION_REGEX");
    static ref BRIDGE_REGEX: Regex = Regex::new(r"^[_\*~]*<(.*?)>[_\*~]* (.*)").expect("Failed to make BRIDGE_REGEX");
    static ref PIKAGIRL_PAY_REGEX: Regex = Regex::new(r"💰.*Transaction of (\d+) coin.*<@!?(\d+)> to <@!?(\d+)>.*").expect("Failed to make PIKAGIRL_PAY_REGEX");
}

pub fn start(main_chain: Sender<ChainMessage>, words: Arc<RwLock<WordMap>>) -> Sender<String> {
    let (sender, reciever): (_, Receiver<String>) = channel();
    let config = DiscordConfig::load("config/discord.json");
    Builder::new()
        .name("discord".to_string())
        .spawn(move || {
            let hyper_client = Arc::new(Client::new());
            //I have no idea how to get a user token normally
            #[allow(deprecated)]
            let discord = Discord::new_cache("config/discord_tokens", &config.login.email, Some(&config.login.password))
                .expect("Login failed");
            let (connection, ready) = Retry::new(&mut || discord.connect(), &mut |result| result.is_ok())
                .wait_between(200, 60000)
                .execute()
                .unwrap()
                .unwrap();
            let state = State::new(ready);
            let name = state.user().username.to_lowercase();
            let (markov_sender, markov_reciever) = channel();
            let mut discord_state = DiscordState {
                main_chain: main_chain,
                state: state,
                server_chains: HashMap::new(),
                channel_chains: HashMap::new(),
                user_chains: HashMap::new(),
                words: words,
                nsfw_words: Arc::new(RwLock::new(WordMap::new())),
                name_map: HashMap::new(),
                name: name,
                markov_sender: markov_sender,
                markov_reciever: markov_reciever,
                config: config,
                discord: discord,
                connection: connection,
                hyper_client: hyper_client,
            };


            let mut last_change = Instant::now();
            let mut change_time = Duration::from_secs(0);
            loop {
                if let Ok(message) = reciever.try_recv() {
                    if "stop" == message.to_lowercase() {
                        discord_state.connection.shutdown().unwrap();
                        break;
                    }
                }
                let event = match discord_state.connection.recv_event() {
                    Ok(event) => event,
                    Err(err) => {
                        println!("[Warning] Receive error: {:?}", err);
                        if let Error::WebSocket(..) = err {
                            let (new_connection, ready) = Retry::new(&mut || discord_state.discord.connect(), &mut |result| result.is_ok())
                                .wait_between(200, 60000)
                                .execute()
                                .unwrap()
                                .unwrap();
                            discord_state.connection = new_connection;
                            discord_state.state = State::new(ready);
                        }
                        continue;
                    }
                };
                discord_state.state.update(&event);
                discord_state.connection.download_all_members(&mut discord_state.state);
                if last_change.elapsed() > change_time {
                    last_change = Instant::now();
                    change_time = Duration::from_secs(UNIX_EPOCH.elapsed().unwrap().as_secs() & 1023);
                    let game_type = change_time.as_secs();
                    discord_state.connection.set_game(if game_type > 0 {
                        discord_state.main_chain.send(ChainMessage::RandomWord(discord_state.markov_sender.clone())).unwrap();
                        let game = discord_state.markov_reciever.recv().unwrap();
                        Some(if game_type == 1 {Game::playing(game)} else {Game::streaming(game.clone(), game)})
                    } else {
                        None
                    });
                    discord_state.connection.sync_servers(&discord_state.state.all_servers());
                }
                if let Event::MessageCreate(message) = event {
                    let message_info = handle_message(&message, &mut discord_state);
                    message_markov(&message_info, &mut discord_state);
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

struct DiscordState {
    state: State,
    main_chain: Sender<ChainMessage>,
    server_chains: HashMap<ServerId, Sender<ChainMessage>>,
    channel_chains: HashMap<ChannelId, Sender<ChainMessage>>,
    user_chains: HashMap<UserId, Sender<ChainMessage>>,
    words: Arc<RwLock<WordMap>>,
    nsfw_words: Arc<RwLock<WordMap>>,
    name_map: HashMap<UserId, UserInfo>,
    name: String,
    markov_sender: Sender<String>,
    markov_reciever: Receiver<String>,
    config: DiscordConfig,
    discord: Discord,
    connection: Connection,
    hyper_client: Arc<Client>,
}

struct MessageInfo {
    private: bool,
    listen: bool,
    content: String,
    author_name: String,
    power: Power,
    channel_id: ChannelId,
    author_id: UserId,
    nsfw: bool,
    participants: Vec<UserId>,
}

#[derive(Default, Debug)]
struct UserInfo {
    name: String,
    nicks: HashMap<ServerId, String>,
}

fn handle_message(message: &Message, discord_state: &mut DiscordState) -> MessageInfo {
    let state = &discord_state.state;
    let main_chain = &mut discord_state.main_chain;
    let server_chains = &mut discord_state.server_chains;
    let channel_chains = &mut discord_state.channel_chains;
    let words = &mut discord_state.words;
    let nsfw_words = &mut discord_state.nsfw_words;
    let name_map = &mut discord_state.name_map;
    let config = &mut discord_state.config;
    let connection = &mut discord_state.connection;
    let hyper_client = &mut discord_state.hyper_client;

    let mut private = false;
    let mut listen = true;
    let mut content = message.content.clone();
    let mut author_name = message.author.name.clone();
    let power = if message.author.id.to_string() == config.owner_id {
        Power::Cool
    } else {
        Power::Normal
    };
    let mut nsfw = false;
    let mut participants = Vec::new();

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
    if content == "$ignore" && power == Power::Cool {
        config.ignored.insert(message.channel_id.0);
        config.save("config/discord.json");
    }
    listen = listen && !config.ignored.contains(&message.channel_id.0);

    if message.author.id != state.user().id {
        match state.find_channel(message.channel_id) {
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
                            let filename = format!("images/discord/{}/{}.{}", server.id, attachment.id, attachment.filename.split('.').last().unwrap());
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
                    chain.set_strength(0.7);
                    chain.thread().0
                });
                for member in &server.members {
                    if let Some(nick) = member.nick.clone() {
                        name_map.entry(member.user.id).or_insert_with(Default::default).nicks.insert(server.id, nick);
                    }
                    name_map.entry(member.user.id).or_insert_with(Default::default).name = member.user.name.clone();
                }
                for presence in &server.presences {
                    if let Some(nick) = presence.nick.clone() {
                        name_map.entry(presence.user_id).or_insert_with(Default::default).nicks.insert(server.id, nick);
                    }
                    if let Some(ref user) = presence.user {
                        name_map.entry(presence.user_id).or_insert_with(Default::default).name = user.name.clone();
                    }
                    if presence.status == OnlineStatus::Online {
                        participants.push(presence.user_id)
                    }
                }
            }
            Some(ChannelRef::Group(group)) => {
                println!("[Discord] [Group {}] {}: {}",
                         group.name(),
                         author_name,
                         content);
                channel_chains.entry(group.channel_id).or_insert_with(|| {
                    let mut chain = MarkovChain::new(words.clone(),
                                                     &format!("lines/discord/groups/{}", group.channel_id));
                    chain.parent = Some(main_chain.clone());
                    chain.thread().0
                });
            }
            Some(ChannelRef::Private(channel)) => {
                println!("[Discord] [Private] {}: {}",
                         author_name,
                         content);
                channel_chains.entry(channel.id).or_insert_with(|| {
                    let mut chain = MarkovChain::new(words.clone(),
                                                     &format!("lines/discord/private/{}", channel.id));
                    chain.parent = Some(main_chain.clone());
                    chain.tell_parent = false;
                    chain.thread().0
                });
                private = true;
            }
            None => {
                println!("[Discord] [Unknown Channel] {}: {}",
                         author_name,
                         content);
                channel_chains.entry(message.channel_id).or_insert_with(|| main_chain.clone());
                private = true;
            }
        }
    }

    MessageInfo {
        private,
        listen,
        content,
        author_name,
        power,
        channel_id: message.channel_id,
        author_id: message.author.id,
        nsfw,
        participants,
    }
}

fn message_markov(message_info: &MessageInfo, discord_state: &mut DiscordState) -> bool {
    if message_info.listen {
        let main_chain = &mut discord_state.main_chain;
        let channel_chains = &mut discord_state.channel_chains;
        let user_chains = &mut discord_state.user_chains;
        let name = &discord_state.name;
        let markov_sender = &mut discord_state.markov_sender;
        let markov_reciever = &mut discord_state.markov_reciever;
        let config = &mut discord_state.config;
        let discord = &mut discord_state.discord;
        let words = &discord_state.words;
        let name_map = &discord_state.name_map;

        let content = &message_info.content;
        let private = message_info.private;
        let nsfw = message_info.nsfw;
        let power = message_info.power;
        let author_name = &message_info.author_name;
        let participants = &message_info.participants;

        let mut users = vec![author_name.clone()];
        for user_id in participants {
            if let Some(user_info) = name_map.get(user_id) {
                users.push(user_info.name.clone());
                for nick in user_info.nicks.values() {
                    users.push(nick.clone());
                }
            }
        }

        let mut chain = &channel_chains[&message_info.channel_id];
        if !(private || nsfw) {
            let user_chain = user_chains.entry(message_info.author_id).or_insert_with(|| {
                let mut chain = MarkovChain::new(words.clone(),
                                                 &format!("lines/discord/users/{}", message_info.author_id));
                chain.set_strength(0.3);
                chain.thread().0
            });
            user_chain.send(ChainMessage::ChangeParent(Some(chain.clone()))).expect("Couldn't change parent");
            chain = user_chain;
        }

        if content.starts_with("$m") && content.len() >= 3 {
            let mut command = content.clone();
            command.drain(..3);
            main_chain.send(ChainMessage::Command(command, power, markov_sender.clone())).expect("Couldn't send Command to chain");
            let _ = discord.send_message(message_info.channel_id,
                                         &markov_reciever.recv().unwrap(),
                                         "",
                                         false);
        } else if content.starts_with("$cm") && content.len() >= 4 {
            let mut command = content.clone();
            command.drain(..4);
            chain.send(ChainMessage::Command(command, power, markov_sender.clone())).unwrap();
            let _ = discord.send_message(message_info.channel_id,
                                         &markov_reciever.recv().unwrap(),
                                         "",
                                         false);
        } else {
            if private || content.to_lowercase().contains(name) || weird_contains(content, name) {
                chain.send(ChainMessage::Reply(content.clone(),
                                               name.clone(),
                                               author_name.clone(),
                                               users.clone(),
                                               markov_sender.clone()))
                    .expect("Couldn't send Reply to chain");
                let response = markov_reciever.recv().unwrap();
                let response = MENTION_REGEX.replace_all(&response, &format!("<@!{}>", config.owner_id)[..]);
                println!("Saying {}", response);
                let _ = discord.send_message(message_info.channel_id,
                                             &response,
                                             "",
                                             false);
            }
            //Nothing good ever comes from lines containing "daddy"
            //TODO make this configurable
            if !content.to_lowercase().contains("daddy") {
                chain.send(ChainMessage::Learn(content.clone(), users)).expect("Couldn't send Learn to chain");
            }
        }
    }
    false
}
