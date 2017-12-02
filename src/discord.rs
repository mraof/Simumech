use discord_lib::{Discord, State, Error, ChannelRef, Connection};
use discord_lib::model::{Event, Message, Game, OnlineStatus, ServerId, ChannelId, UserId, User};
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
use std::time::{Instant, Duration, UNIX_EPOCH};
use super::regex::Regex;
use super::serde_json;
use tcpgen::TCPList;
use morbitgen::{Template, Requirement};
use linked_hash_map::LinkedHashMap;
use super::letters_only;

lazy_static! {
    static ref MENTION_REGEX: Regex = Regex::new(r"@here|@everyone|<@!?\d+>").expect("Failed to make MENTION_REGEX");
    static ref IRC_BRIDGE_REGEX: Regex = Regex::new(r"^[_\*~]*<(.*?)>[_\*~]* (.*)").expect("Failed to make IRC_BRIDGE_REGEX");
    static ref PIKAGIRL_BRIDGE_REGEX: Regex = Regex::new(r"^. [_\*~]*(.*?):[_\*~]* (.*)").expect("Failed to make PIKAGIRL_BRIDGE_REGEX");
    static ref PIKAGIRL_PAY_REGEX: Regex = Regex::new(r"💰.*Transaction of (\d+) coin.*<@!?(\d+)> to <@!?(\d+)>.*").expect("Failed to make PIKAGIRL_PAY_REGEX");
    static ref DICE_REGEX: Regex = Regex::new(r"(?P<c>\d{0,4})d(?P<d>\d{1,16})").unwrap();
    static ref NO_COUNT_REGEX: Regex = Regex::new(r"d\(,").unwrap();
}

pub fn start(main_chain: Sender<ChainMessage>, words: Arc<RwLock<WordMap>>) -> Sender<String> {
    let (sender, reciever): (_, Receiver<String>) = channel();
    let config = DiscordConfig::load("config/discord.json");
    Builder::new()
        .name("discord".to_string())
        .spawn(move || {
            //I have no idea how to get a user token normally
            #[allow(deprecated)]
            let discord = Discord::new_cache(
                "config/discord_tokens",
                &config.login.email,
                Some(&config.login.password),
            ).expect("Login failed");
            let (connection, ready) = Retry::new(&mut || discord.connect(), &mut |result| result.is_ok())
                .wait_between(200, 60000)
                .execute()
                .unwrap()
                .unwrap();
            let state = State::new(ready);
            let name = state.user().username.to_lowercase();
            let functions = [roll, tcp, gen, misc, message_markov];
            let (markov_sender, markov_reciever) = channel();
            let mut discord_state = DiscordState {
                main_chain,
                state,
                chains: Default::default(),
                words,
                nsfw_words: Arc::new(RwLock::new(WordMap::new())),
                name,
                markov_sender,
                markov_reciever,
                tcp_list: TCPList::new(&config.tcpdir),
                config,
                discord,
                connection,
                generator: Generator {
                    generators: load_generators(),
                    generated: Default::default(),
                },
            };


            let mut last_change = Instant::now();
            let mut change_time = Duration::from_secs(0);
            loop {
                if let Ok(message) = reciever.try_recv() {
                    if "stop" == message.to_lowercase() {
                        discord_state.connection.shutdown().unwrap();
                        discord_state.config.save("config/discord.json");
                        discord_state.chains.stop();
                        break;
                    } else if "reload" == message.to_lowercase() {
                        discord_state.tcp_list = TCPList::new(&discord_state.config.tcpdir);
                        discord_state.generator.generators = load_generators();
                    }
                }
                let event = match discord_state.connection.recv_event() {
                    Ok(event) => event,
                    Err(err) => {
                        println!("[Warning] Receive error: {:?}", err);
                        if let Error::WebSocket(..) = err {
                            let (new_connection, ready) = Retry::new(&mut || discord_state.discord.connect(), &mut |result| {
                                result.is_ok()
                            }).wait_between(200, 60000)
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
                discord_state.connection.download_all_members(
                    &mut discord_state.state,
                );
                if last_change.elapsed() > change_time {
                    last_change = Instant::now();
                    change_time = Duration::from_secs(UNIX_EPOCH.elapsed().unwrap().as_secs() & 1023);
                    let game_type = change_time.as_secs();
                    discord_state.connection.set_game(if game_type > 0 {
                        discord_state
                            .main_chain
                            .send(ChainMessage::RandomWord(
                                discord_state.markov_sender.clone(),
                            ))
                            .unwrap();
                        let game = discord_state.markov_reciever.recv().unwrap();
                        Some(if game_type == 1 {
                            Game::playing(game)
                        } else {
                            Game::streaming(game.clone(), game)
                        })
                    } else {
                        None
                    });
                    discord_state.connection.sync_servers(
                        &discord_state.state.all_servers(),
                    );
                }
                if let Event::MessageCreate(message) = event {
                    let message_info = handle_message(&message, &mut discord_state);
                    for function in &functions {
                        if function(&message_info, &mut discord_state) {
                            break;
                        }
                    }
                }
            }
        })
        .unwrap();
    sender
}

fn distance_contains(message: &str, target: &str, limit: usize) -> bool {
    use distance::damerau_levenshtein;

    let full = letters_only(message);
    if full.len() <= target.len() {
        return damerau_levenshtein(&full, target) <= limit
    }
    for i in 0..(full.len() - target.len() + 1) {
        if damerau_levenshtein(&full[i..(i + target.len())], target) <= limit {
            return true
        }
    }
    false
}

#[derive(Default, Debug, Deserialize, Serialize)]
#[serde(default)]
struct DiscordConfig {
    login: DiscordLogin,
    owner_id: String,
    ignored: HashSet<u64>,
    users: HashMap<u64, UserInfo>,
    tcpdir: String,
}

#[derive(Default, Debug, Deserialize, Serialize)]
#[serde(default)]
struct DiscordLogin {
    email: String,
    password: String,
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
        serde_json::to_writer_pretty(
            &mut File::create(filename).expect("Could not create discord config file"),
            &self,
        ).expect("Failed to write discord config file");
    }
}

struct DiscordState {
    state: State,
    main_chain: Sender<ChainMessage>,
    chains: ChainManager,
    words: Arc<RwLock<WordMap>>,
    nsfw_words: Arc<RwLock<WordMap>>,
    name: String,
    markov_sender: Sender<String>,
    markov_reciever: Receiver<String>,
    config: DiscordConfig,
    discord: Discord,
    connection: Connection,
    tcp_list: TCPList,
    generator: Generator,
}

struct MessageInfo {
    private: bool,
    listen: bool,
    content: String,
    author_name: String,
    power: Power,
    channel_type: ChannelType,
    channel_id: ChannelId,
    author_id: UserId,
    nsfw: bool,
    participants: Vec<UserId>,
    mentions: Vec<User>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
struct UserInfo {
    name: String,
    //TODO change back to ServerId
    nicks: HashMap<u64, String>,
}

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone)]
enum DiscordId {
    Server(ServerId),
    Channel(ChannelId),
    User(UserId),
}

impl From<ServerId> for DiscordId {
    fn from(id: ServerId) -> Self {
        DiscordId::Server(id)
    }
}

impl From<ChannelId> for DiscordId {
    fn from(id: ChannelId) -> Self {
        DiscordId::Channel(id)
    }
}

impl From<UserId> for DiscordId {
    fn from(id: UserId) -> Self {
        DiscordId::User(id)
    }
}

#[derive(Debug, Copy, Clone)]
enum ChannelType {
    Server(ServerId),
    Group,
    Private,
    Unknown,
}

fn handle_message(message: &Message, discord_state: &mut DiscordState) -> MessageInfo {
    let state = &discord_state.state;
    let config = &mut discord_state.config;
    let connection = &mut discord_state.connection;

    let mut private = false;
    let mut listen = true;
    let mut content = message.content.clone();
    let mut author_name = message.author.name.clone();
    let mentions = message.mentions.clone();
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
                if let Some(cap) = PIKAGIRL_BRIDGE_REGEX.captures(&message.content) {
                    author_name = cap.get(1).expect("No author captured").as_str().to_string();
                    content = cap.get(2)
                        .expect("No content captured")
                        .as_str()
                        .to_string();
                    listen = true;
                } else if let Some(cap) = PIKAGIRL_PAY_REGEX.captures(&message.content) {
                    println!("Gambling! {:?}", message.mentions);
                    let amount = cap.get(1).expect("No amount captured");
                    let payer = cap.get(2).expect("No payer captured");
                    let paid = cap.get(3).expect("No paid captured");
                    if paid.as_str().parse::<u64>().expect(
                        "User id wasn't a number",
                    ) == state.user().id.0
                    {
                        println!("{} paid {} to me~", payer.as_str(), amount.as_str());
                    }
                }
            }
            //crocbot
            229824641160577025 => {
                if let Some(cap) = IRC_BRIDGE_REGEX.captures(&message.content) {
                    author_name = cap.get(1).expect("No author captured").as_str().to_string();
                    content = cap.get(2)
                        .expect("No content captured")
                        .as_str()
                        .to_string();
                    listen = true;
                }
            }
            _ => {
                println!("Unhandled bot {}", author_name);
            }
        }
    }
    listen = listen && !config.ignored.contains(&message.channel_id.0) && message.author.id != state.user().id;

    let users = &mut config.users;
    let channel_type = match state.find_channel(message.channel_id) {
        Some(ChannelRef::Public(server, channel)) => {
            println!(
                "[Discord] [{} #{}] {}: {}",
                server.name,
                channel.name,
                author_name,
                content
            );
            nsfw = channel.name.contains("nsfw") || channel.nsfw;
            connection.sync_servers(&[server.id]);
            for member in &server.members {
                if let Some(nick) = member.nick.clone() {
                    users
                        .entry(member.user.id.0)
                        .or_insert_with(Default::default)
                        .nicks
                        .insert(server.id.0, nick);
                }
                users
                    .entry(member.user.id.0)
                    .or_insert_with(Default::default)
                    .name = member.user.name.clone();
            }
            for presence in &server.presences {
                if let Some(nick) = presence.nick.clone() {
                    users
                        .entry(presence.user_id.0)
                        .or_insert_with(Default::default)
                        .nicks
                        .insert(server.id.0, nick);
                }
                if let Some(ref user) = presence.user {
                    users
                        .entry(presence.user_id.0)
                        .or_insert_with(Default::default)
                        .name = user.name.clone();
                }
                if presence.status == OnlineStatus::Online {
                    participants.push(presence.user_id)
                }
            }
            ChannelType::Server(server.id)
        }
        Some(ChannelRef::Group(group)) => {
            println!(
                "[Discord] [Group {}] {}: {}",
                group.name(),
                author_name,
                content
            );
            for user in &group.recipients {
                users.entry(user.id.0).or_insert_with(Default::default).name = user.name.clone();
                participants.push(user.id)
            }
            ChannelType::Group
        }
        Some(ChannelRef::Private(_)) => {
            println!("[Discord] [Private] {}: {}", author_name, content);
            private = true;
            ChannelType::Private
        }
        None => {
            println!("[Discord] [Unknown Channel] {}: {}", author_name, content);
            private = true;
            ChannelType::Unknown
        }
    };

    MessageInfo {
        private,
        listen,
        content,
        author_name,
        power,
        channel_type,
        channel_id: message.channel_id,
        author_id: message.author.id,
        nsfw,
        participants,
        mentions,
    }
}

fn message_markov(message_info: &MessageInfo, discord_state: &mut DiscordState) -> bool {
    if message_info.listen {
        let main_chain = &mut discord_state.main_chain;
        let chains = &mut discord_state.chains;
        let name = &discord_state.name;
        let markov_sender = &mut discord_state.markov_sender;
        let markov_reciever = &mut discord_state.markov_reciever;
        let config = &mut discord_state.config;
        let discord = &mut discord_state.discord;
        let words = &discord_state.words;
        let nsfw_words = &mut discord_state.nsfw_words;

        let content = &message_info.content;
        let private = message_info.private;
        let nsfw = message_info.nsfw;
        let power = message_info.power;
        let channel_type = message_info.channel_type;
        let channel_id = message_info.channel_id;
        let author_name = &message_info.author_name;
        let participants = &message_info.participants;

        let mut names = vec![author_name.clone()];
        for user_id in participants {
            if let Some(user_info) = config.users.get(&user_id.0) {
                names.push(user_info.name.clone());
                for nick in user_info.nicks.values() {
                    names.push(nick.clone());
                }
            }
        }

        let parent_chain = match channel_type {
            ChannelType::Server(server_id) => {
                chains
                    .get(server_id.into(), || {
                        let mut chain = MarkovChain::new(
                            words.clone(),
                            &format!("lines/discord/{}/server", server_id),
                        );
                        chain.parent = Some(main_chain.clone());
                        chain.thread().0
                    })
                    .clone()
            }
            _ => main_chain.clone(),
        };
        let mut chain = chains
            .get(channel_id.into(), || {
                let dir = match channel_type {
                    ChannelType::Server(server_id) => server_id.0.to_string(),
                    ChannelType::Group => "groups".to_string(),
                    ChannelType::Private => "private".to_string(),
                    ChannelType::Unknown => "unknown".to_string(),
                };
                let mut chain = MarkovChain::new(
                    if nsfw {
                        nsfw_words.clone()
                    } else {
                        words.clone()
                    },
                    &format!("lines/discord/{}/{}", dir, channel_id),
                );
                match channel_type {
                    ChannelType::Server(_) => {
                        chain.set_strength(0.7);
                    }
                    _ => (),
                };
                chain.parent = Some(parent_chain);
                if nsfw || private {
                    chain.tell_parent = false
                }
                chain.thread().0
            })
            .clone();

        if !(private || nsfw) {
            let user_chain = chains.get(message_info.author_id.into(), || {
                let mut chain = MarkovChain::new(
                    words.clone(),
                    &format!("lines/discord/users/{}", message_info.author_id),
                );
                chain.set_strength(0.3);
                chain.thread().0
            });
            user_chain
                .send(ChainMessage::ChangeParent(Some(chain.clone())))
                .expect("Couldn't change parent");
            chain = user_chain;
        }

        if content.starts_with("$m") && content.len() >= 3 {
            let mut command = content.clone();
            command.drain(..3);
            main_chain
                .send(ChainMessage::Command(command, power, markov_sender.clone()))
                .expect("Couldn't send Command to chain");
            let _ = discord.send_message(channel_id, &markov_reciever.recv().unwrap(), "", false);
        } else if content.starts_with("$cm") && content.len() >= 4 {
            let mut command = content.clone();
            command.drain(..4);
            chain
                .send(ChainMessage::Command(command, power, markov_sender.clone()))
                .unwrap();
            let _ = discord.send_message(channel_id, &markov_reciever.recv().unwrap(), "", false);
        } else {
            if private || distance_contains(content, name, 3) {
                chain
                    .send(ChainMessage::Reply(
                        content.clone(),
                        name.clone(),
                        author_name.clone(),
                        names.clone(),
                        markov_sender.clone(),
                    ))
                    .expect("Couldn't send Reply to chain");
                let response = markov_reciever.recv().unwrap();
                let response = MENTION_REGEX.replace_all(&response, &format!("<@!{}>", config.owner_id)[..]);
                println!("Saying {}", response);
                let _ = discord.send_message(channel_id, &response, "", false);
            }
            //Nothing good ever comes from lines containing "daddy"
            //TODO make this configurable
            if !content.to_lowercase().contains("daddy") {
                chain
                    .send(ChainMessage::Learn(content.clone(), names))
                    .expect("Couldn't send Learn to chain");
            }
        }
        chains.clean();
    }
    false
}

fn roll(message_info: &MessageInfo, discord_state: &mut DiscordState) -> bool {
    use meval;
    use rand::Rng;
    use rand::OsRng;
    use std::iter::Sum;
    let discord = &discord_state.discord;
    let arguments: Vec<&str> = message_info.content.splitn(2, ' ').collect();
    if arguments[0] == "$roll" {
        let expr = if arguments.len() == 2 {
            arguments[1]
        } else {
            "d12"
        };
        let arguments: Vec<&str> = expr.splitn(2, ':').collect();
        let expr = DICE_REGEX.replace_all(arguments[0], "d($c, $d)");
        let expr = NO_COUNT_REGEX.replace_all(&expr, "d(1,");
        println!("{}", expr);
        let message = match expr.parse::<meval::Expr>() {
            Ok(expr) => {
                let mut ctx = meval::Context::new();
                ctx.func2("d", |c, d| {
                    if d == 0.0 {
                        return 0.0;
                    }
                    let mut random = OsRng::new().unwrap();
                    let mut rolls = Vec::new();
                    for _ in 0..(c as usize) {
                        rolls.push((random.next_u64() % (d as u64) + 1) as f64)
                    }
                    Sum::sum(rolls.into_iter())
                });
                match expr.eval_with_context(ctx) {
                    Ok(result) => {
                        format!(
                            "`{}{}`",
                            result,
                            if arguments.len() >= 2 {
                                format!(":{}", arguments[1])
                            } else {
                                "".to_string()
                            }
                        )
                    }
                    Err(error) => format!("```{:?}\n{:?}```", expr, error),
                }
            }
            Err(error) => format!("```{}\n{:?}```", expr, error),
        };
        let _ = discord.send_message(message_info.channel_id, &message, "", false);
        true
    } else {
        false
    }
}

fn tcp(message_info: &MessageInfo, discord_state: &mut DiscordState) -> bool {
    let discord = &discord_state.discord;
    let tcp_list = &discord_state.tcp_list;
    let arguments: Vec<&str> = message_info.content.splitn(2, ' ').collect();
    if arguments[0].to_lowercase() == "$tcp" && message_info.listen {
        if arguments.len() > 1 && arguments[1].to_lowercase() == "verbose" {
            let _ = discord.send_message(
                message_info.channel_id,
                &format!("{:#?}", tcp_list.gen()),
                "",
                false,
            );
        } else {
            let _ = discord.send_message(
                message_info.channel_id,
                &format!("{}", tcp_list.gen()),
                "",
                false,
            );
        }
        true
    } else {
        false
    }
}

struct Generator {
    generators: HashMap<String, Template>,
    generated: HashMap<UserId, (HashMap<String, String>, String)>,
}

fn gen(message_info: &MessageInfo, discord_state: &mut DiscordState) -> bool {
    let arguments: Vec<&str> = message_info.content.splitn(2, ' ').collect();
    if !arguments[0].starts_with('$') || !message_info.listen {
        return false;
    }
    let discord = &discord_state.discord;
    let generators = &discord_state.generator.generators;
    let generated = &mut discord_state.generator.generated;
    let command = &arguments[0][1..];
    println!("Recieved command \"{}\"", command);
    let mut template = generators.get(command);
    let mut presets = Vec::new();
    let mut formatting = "full".to_string();
    if template.is_some() {
        let arguments = arguments.get(1).map_or("", |argument| *argument);
        if arguments.contains(':') && (!arguments.contains('[') || arguments.find('?').unwrap_or(arguments.len()) < arguments.find('[').unwrap()) {
            let mut split = arguments.splitn(2, '?');
            presets = split
                .next()
                .unwrap()
                .split(',')
                .map(|preset| preset.parse().unwrap())
                .collect();
            if let Some(argument) = split.next() {
                formatting = argument.to_string();
            }
        } else if !arguments.trim().is_empty() {
            formatting = arguments.to_string();
        }
    } else if command == "regen" && arguments.len() == 2 {
        if let Some(generated) = generated.get(&message_info.author_id) {
            formatting = generated.1.clone();
            let mut values = generated.0.clone();
            template = generators.get(values.get("species").unwrap_or(&"base".to_string()));
            if let Some(template) = template {
                for arg in arguments[1].split(',').map(|arg| arg.trim()) {
                    if arg.contains(':') {
                        presets.push(arg.parse().unwrap());
                        if !arg.contains('|') {
                            values.remove(arg.splitn(2, ':').next().unwrap());
                        }
                    } else {
                        values.remove(arg);
                    }
                }
                for (key, value) in values {
                    if !template.always(&key, &value) {
                        presets.push(Requirement { possibilities: vec![(key, value, false)] });
                    }
                }
            }
        }
    }
    if let Some(template) = template {
        let gen = template.generate(presets);
        let output = match template.format(&gen, &formatting) {
            Ok(mut output) => {
                if formatting == "json" {
                    output = format!("```json\n{}\n```", output);
                }
                output
            }
            Err(error) => error,
        };
        let output = MENTION_REGEX.replace_all(
            &output,
            &format!("<@!{}>", discord_state.config.owner_id)[..],
        );
        generated.insert(message_info.author_id, (gen, formatting));
        let _ = discord.send_message(message_info.channel_id, &output, "", false);
        true
    } else {
        false
    }
}

fn load_generators() -> HashMap<String, Template> {
    let mut generators = HashMap::new();
    let base_generator = Template::new("base", None);
    generators.insert(
        "obj".to_string(),
        Template::new("obj", Some(&base_generator)),
    );
    generators.insert("base".to_string(), base_generator);
    generators
}

fn misc(message_info: &MessageInfo, discord_state: &mut DiscordState) -> bool {
    if message_info.content.starts_with('$') {
        let mut split = message_info.content.splitn(2, ' ');
        let command = split.next().unwrap().to_lowercase();
        let args = split.next().unwrap_or_default();
        match &command[1..] {
            "ignore" if message_info.power == Power::Cool => {
                discord_state.config.ignored.insert(
                    message_info.channel_id.0,
                );
                discord_state.config.save("config/discord.json");
                true
            }
            "names" => {
                let user_id = if message_info.mentions.is_empty() {
                    args.parse()
                } else {
                    Ok(message_info.mentions[0].id.0)
                };
                if let Ok(user_id) = user_id {
                    let response = if let Some(user_info) = discord_state.config.users.get(&user_id) {
                        let mut names = user_info.name.clone();
                        for name in user_info.nicks.values() {
                            names += "\n";
                            names += name;
                        }
                        names
                    } else {
                        format!("User id {} is unknown", user_id)
                    };
                    let _ = discord_state.discord.send_message(
                        message_info.channel_id,
                        &response,
                        "",
                        false,
                    );
                }
                true
            }
            _ => false,
        }
    } else {
        false
    }
}

#[derive(Default)]
struct ChainManager {
    chains: HashMap<DiscordId, Sender<ChainMessage>>,
    ages: LinkedHashMap<DiscordId, Instant>,
}

impl ChainManager {
    fn get<F: FnOnce() -> Sender<ChainMessage>>(&mut self, id: DiscordId, default: F) -> Sender<ChainMessage> {
        self.ages.insert(id, Instant::now());
        self.chains.entry(id).or_insert_with(default).clone()
    }

    fn clean(&mut self) {
        while self.ages.front().map_or(false, |(_, instant)| {
            instant.elapsed() > Duration::from_secs(60 * 30)
        })
        {
            let (id, _) = self.ages.pop_front().unwrap();
            let chain = self.chains.remove(&id).unwrap();
            chain.send(ChainMessage::Stop).expect(
                "Failed to send stop message",
            );
        }
    }

    fn stop(self) {
        for (_, chain) in self.chains.into_iter() {
            chain.send(ChainMessage::Stop).expect(
                "Failed to send stop message",
            );
        }
    }
}
