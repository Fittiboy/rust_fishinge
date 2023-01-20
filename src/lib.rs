use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};
use std::collections::HashMap;
use std::fs::read_to_string;
use std::thread::sleep;
use std::time::Duration;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    client_id: String,
    streamer: String,
    user_access_token: String,
    reward_title: String,
    jwt: String,
    command_name: String,
}

impl Config {
    pub fn load() -> Config {
        let mut config_file = dirs::config_dir().expect("there should always be a config dir");
        config_file.push("fishinge");
        config_file.push("fishinge.conf");
        let config_data = read_to_string(config_file).unwrap();
        let config: Config = toml::from_str(&config_data).unwrap();
        config
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    pub fn streamer(&self) -> &str {
        &self.streamer
    }

    pub fn user_access_token(&self) -> &str {
        &self.user_access_token
    }

    pub fn reward_title(&self) -> &str {
        &self.reward_title
    }

    pub fn jwt(&self) -> &str {
        &self.jwt
    }

    pub fn command_name(&self) -> &str {
        &self.command_name
    }
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct BroadcasterResponse {
    data: Vec<BroadcasterData>,
    pagination: HashMap<String, String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct BroadcasterData {
    broadcaster_language: String,
    broadcaster_login: String,
    display_name: String,
    game_id: String,
    game_name: String,
    id: String,
    is_live: bool,
    tag_ids: Vec<String>,
    tags: Vec<String>,
    thumbnail_url: String,
    title: String,
    started_at: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct RewardResponse {
    data: Vec<RewardData>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct RewardData {
    broadcaster_name: String,
    broadcaster_login: String,
    broadcaster_id: String,
    id: String,
    image: Option<String>,
    background_color: String,
    is_enabled: bool,
    cost: Number,
    title: String,
    prompt: Option<String>,
    is_user_input_required: bool,
    max_per_stream_setting: Map<String, Value>,
    max_per_user_per_stream_setting: Map<String, Value>,
    global_cooldown_setting: Map<String, Value>,
    is_paused: bool,
    is_in_stock: bool,
    default_image: Map<String, Value>,
    should_redemptions_skip_request_queue: bool,
    redemptions_redeemed_current_stream: Option<Number>,
    cooldown_expires_at: Option<String>,
}

pub async fn get_ids(config: &Config) -> Result<(String, String), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let res: BroadcasterResponse = client
        .get(format!(
            "https://api.twitch.tv/helix/search/channels?query={}",
            config.streamer()
        ))
        .header(
            "Authorization",
            format!("Bearer {}", config.user_access_token()),
        )
        .header("Client-Id", config.client_id())
        .send()
        .await?
        .json::<BroadcasterResponse>()
        .await?;

    let mut broadcaster_id: String = String::new();
    for broadcaster in res.data {
        if broadcaster.broadcaster_login == config.streamer() {
            broadcaster_id = broadcaster.id;
            break;
        }
    }
    if broadcaster_id.is_empty() {
        return Err("oh no, no id found".into());
    }
    let res: RewardResponse = client
        .get(format!(
            "https://api.twitch.tv/helix/channel_points/custom_rewards?broadcaster_id={}",
            broadcaster_id,
        ))
        .header(
            "Authorization",
            format!("Bearer {}", config.user_access_token()),
        )
        .header("Client-Id", config.client_id())
        .send()
        .await?
        .json()
        .await?;

    let mut reward_id = String::new();
    for reward in res.data {
        if reward.title == config.reward_title() {
            reward_id = reward.id;
            break;
        }
    }

    if reward_id.is_empty() {
        return Err("reward not found".into());
    };

    Ok((broadcaster_id, reward_id))
}

#[derive(Serialize, Debug)]
#[allow(dead_code)]
struct RequestBody {
    r#type: String,
    version: String,
    condition: RewardCondition,
    transport: Transport,
}

#[derive(Serialize, Debug)]
#[allow(dead_code)]
struct RewardCondition {
    broadcaster_user_id: String,
    reward_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[allow(dead_code)]
struct Transport {
    method: String,
    session_id: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct SubscriptionResponse {
    data: Vec<SubscriptionData>,
    total: Number,
    total_cost: Number,
    max_total_cost: Number,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct SubscriptionData {
    id: String,
    status: String,
    r#type: String,
    version: String,
    condition: Value,
    created_at: String,
    transport: Transport,
    cost: Number,
}

pub async fn create_subscription(
    config: &Config,
    session_id: String,
    broadcaster_id: String,
    reward_id: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let request_body = RequestBody {
        r#type: "channel.channel_points_custom_reward_redemption.add".into(),
        version: "1".into(),
        condition: RewardCondition {
            broadcaster_user_id: broadcaster_id,
            reward_id,
        },
        transport: Transport {
            method: "websocket".into(),
            session_id,
        },
    };

    client
        .post("https://api.twitch.tv/helix/eventsub/subscriptions")
        .header(
            "Authorization",
            format!("Bearer {}", config.user_access_token()),
        )
        .header("Client-Id", config.client_id())
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?
        .json::<SubscriptionResponse>()
        .await?;

    Ok(())
}

#[derive(Deserialize, Debug)]
#[allow(dead_code, non_snake_case)]
struct AccessResponse {
    channelId: String,
    username: String,
    avatar: String,
    provider: String,
    role: String,
}

#[derive(Deserialize, Serialize, Debug, Default)]
#[allow(dead_code, non_snake_case)]
struct CommandResponse {
    cooldown: Cooldown,
    aliases: Vec<String>,
    keywords: Vec<String>,
    enabled: bool,
    enabledOnline: bool,
    enabledOffline: bool,
    hidden: bool,
    cost: i32,
    r#type: String,
    accessLevel: i32,
    _id: String,
    regex: Option<String>,
    reply: String,
    command: String,
    channel: String,
    createdAt: String,
    updatedAt: String,
}

#[derive(Deserialize, Serialize, Debug, Default)]
#[allow(dead_code)]
struct Cooldown {
    user: i32,
    global: i32,
}

pub async fn update_command(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let url_base = "https://api.streamelements.com/kappa/v2/";
    let client = reqwest::Client::new();
    let res = client
        .get(url_base.to_string() + "users/access")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("Authorization", format!("Bearer {}", config.jwt()))
        .send()
        .await?
        .json::<Vec<AccessResponse>>()
        .await?;

    let mut channel_id = String::new();
    for res in res {
        if res.username == config.streamer() {
            channel_id = res.channelId;
            break;
        }
    }

    if channel_id.is_empty() {
        return Err("channel_id not found".into());
    };

    let res = client
        .get(url_base.to_string() + &format!("bot/commands/{}", channel_id))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("Authorization", format!("Bearer {}", config.jwt()))
        .send()
        .await?
        .json::<Vec<CommandResponse>>()
        .await?;

    let mut command = CommandResponse::default();
    for res in res {
        if res.command == config.command_name() {
            command = res;
            break;
        }
    }

    if command.command.is_empty() {
        return Err("command not found".into());
    }

    command.enabledOnline = true;

    let mut command = client
        .put(url_base.to_string() + &format!("bot/commands/{}/{}", channel_id, command._id))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("Authorization", format!("Bearer {}", config.jwt()))
        .json(&command)
        .send()
        .await?
        .json::<CommandResponse>()
        .await?;

    if command.command.is_empty() {
        return Err("command not enabled correctly".into());
    }

    println!("Enabled command!");
    println!("Waiting 5 minutes...");

    sleep(Duration::from_secs(300));

    command.enabledOnline = false;

    let command = client
        .put(url_base.to_string() + &format!("bot/commands/{}/{}", channel_id, command._id))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("Authorization", format!("Bearer {}", config.jwt()))
        .json(&command)
        .send()
        .await?
        .json::<CommandResponse>()
        .await?;

    if command.command.is_empty() {
        return Err("command not disabled correctly".into());
    }

    println!("Disabled command!");
    Ok(())
}
