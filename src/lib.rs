use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};
use std::collections::HashMap;
use std::fs::{read_to_string, DirBuilder, OpenOptions};
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub client_id: String,
    pub streamer: String,
    pub user_access_token: String,
    pub reward_title: String,
    pub jwt: String,
    pub command_name: String,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            client_id: "".to_owned(),
            streamer: "chobo".to_owned(),
            user_access_token: "".to_owned(),
            reward_title: "5 Minute Fishing Trip".to_owned(),
            jwt: "".to_owned(),
            command_name: "fishinge".to_owned(),
        }
    }
}

impl Config {
    fn get_filepath() -> Result<std::path::PathBuf> {
        let mut config_dir =
            dirs::config_dir().ok_or_else(|| anyhow!("could not find config dir"))?;
        config_dir.push("fishinge");
        Ok(config_dir)
    }

    pub fn load() -> Result<Config> {
        let mut config_file = Config::get_filepath()?;
        config_file.push("fishinge.conf");
        let config_data = read_to_string(&config_file)
            .with_context(|| format!("Failed to read config file from {:?}", &config_file))?;
        toml::from_str(&config_data)
            .with_context(|| format!("Could not read config data from string: {}", &config_data))
    }

    pub fn write(&self) -> Result<()> {
        let mut config_file = Config::get_filepath()?;
        DirBuilder::new()
            .recursive(true)
            .create(config_file.clone())?;
        config_file.push("fishinge.conf");
        let mut file_handle = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(config_file)?;
        file_handle.write_all(toml::to_string_pretty(&self)?.as_bytes())?;
        file_handle.flush()?;
        Ok(())
    }

    pub fn empty() -> Config {
        Config {
            ..Default::default()
        }
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

pub fn write_output(output: &Arc<Mutex<String>>, text: &str) -> Result<()> {
    match output.lock() {
        Ok(mut out) => out.push_str(&format!("{}\n", text)),
        _ => return Err(anyhow!("could not acquire clean lock on output")),
    }
    Ok(())
}

pub fn get_ids(config: &Config) -> Result<(String, String)> {
    let client = reqwest::blocking::Client::new();
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
        .with_context(|| {
            format!(
                "Failed sending request to get broadcaster ID of {}",
                config.streamer()
            )
        })?
        .error_for_status()?
        .json::<BroadcasterResponse>()
        .context("Failed to parse response for broadcaster ID request")?;

    let mut broadcaster_id: String = String::new();
    for broadcaster in res.data {
        if broadcaster.broadcaster_login == config.streamer() {
            broadcaster_id = broadcaster.id;
            break;
        }
    }
    if broadcaster_id.is_empty() {
        return Err(anyhow!("oh no, no id found"));
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
        .with_context(|| {
            format!(
                "Failed sending request to get rewards of {}",
                broadcaster_id,
            )
        })?
        .json::<RewardResponse>()
        .context("Failed to parse response for rewards list request")?;

    let mut reward_id = String::new();
    for reward in res.data {
        if reward.title == config.reward_title() {
            reward_id = reward.id;
            break;
        }
    }

    if reward_id.is_empty() {
        return Err(anyhow!("reward not found"));
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

pub fn create_subscription(
    config: &Config,
    session_id: String,
    broadcaster_id: String,
    reward_id: String,
) -> Result<()> {
    let client = reqwest::blocking::Client::new();
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
        .with_context(|| {
            format!(
                "Failed sending request to create subscription, with body: {:#?}",
                request_body,
            )
        })?
        .json::<SubscriptionResponse>()
        .context("Failed to parse response for subscription request")?;

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

pub fn update_command(output: &Arc<Mutex<String>>, config: &Config) -> Result<()> {
    let url_base = "https://api.streamelements.com/kappa/v2/";
    let client = reqwest::blocking::Client::new();
    let res: Vec<AccessResponse> = client
        .get(url_base.to_string() + "users/access")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("Authorization", format!("Bearer {}", config.jwt()))
        .send()
        .context("Failed sending request to update list of users")?
        .json::<Vec<AccessResponse>>()
        .context("Failed to parse response for user list request")?;

    let mut channel_id = String::new();
    for res in res {
        if res.username == config.streamer() {
            channel_id = res.channelId;
            break;
        }
    }

    if channel_id.is_empty() {
        return Err(anyhow!("channel_id not found"));
    };

    let res: Vec<CommandResponse> = client
        .get(url_base.to_string() + &format!("bot/commands/{}", channel_id))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("Authorization", format!("Bearer {}", config.jwt()))
        .send()
        .context("Failed sending request to get command list")?
        .json::<Vec<CommandResponse>>()
        .context("Failed to parse response for command list request")?;

    let mut command = CommandResponse::default();
    for res in res {
        if res.command == config.command_name() {
            command = res;
            break;
        }
    }

    if command.command.is_empty() {
        return Err(anyhow!("command \"{}\" not found", config.command_name()));
    }

    command.enabledOnline = true;

    let mut command = client
        .put(url_base.to_string() + &format!("bot/commands/{}/{}", channel_id, command._id))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("Authorization", format!("Bearer {}", config.jwt()))
        .json(&command)
        .send()
        .context("Failed sending request to enable command")?
        .json::<CommandResponse>()
        .context("Failed to parse response for command enabling request")?;

    if command.command.is_empty() {
        return Err(anyhow!("command not enabled correctly"));
    }

    write_output(output, "Enabled command!")?;
    write_output(output, "Waiting 5 minutes...")?;

    sleep(Duration::from_secs(300));

    command.enabledOnline = false;

    let command = client
        .put(url_base.to_string() + &format!("bot/commands/{}/{}", channel_id, command._id))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("Authorization", format!("Bearer {}", config.jwt()))
        .json(&command)
        .send()
        .context("Failed sending request to disable command")?
        .json::<CommandResponse>()
        .context("Failed to parse response for command disabling request")?;

    if command.command.is_empty() {
        return Err(anyhow!("command not disabled correctly"));
    }

    write_output(output, "Disabled command!")?;
    Ok(())
}
