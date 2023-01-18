use serde::Deserialize;
use serde_json::{Map, Number, Value};
use std::collections::HashMap;
use std::fs::read_to_string;

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
    redemptions_redeemed_current_stream: Option<String>,
    cooldown_expires_at: Option<String>,
}

pub async fn get_reward_id() -> Result<String, Box<dyn std::error::Error>> {
    let streamer = read_to_string("./streamer.txt").expect("needs to have streamer.txt present");
    let token = read_to_string("./uat.txt").expect("needs to have uat.txt present");
    let client_id = read_to_string("./client_id.txt").expect("needs to have client_id.txt present");
    let client = reqwest::Client::new();
    let res: BroadcasterResponse = client
        .get(format!(
            "https://api.twitch.tv/helix/search/channels?query={}",
            streamer.trim()
        ))
        .header("Authorization", format!("Bearer {}", token.trim()))
        .header("Client-Id", client_id.trim())
        .send()
        .await?
        .json::<BroadcasterResponse>()
        .await?;

    let mut broadcaster_id: String = String::new();
    for broadcaster in res.data {
        if broadcaster.broadcaster_login == streamer.trim() {
            broadcaster_id = broadcaster.id;
            break;
        }
    }
    if broadcaster_id.len() == 0 {
        return Err("oh no, no id found".into());
    }
    let res: RewardResponse = client
        .get(format!(
            "https://api.twitch.tv/helix/channel_points/custom_rewards?broadcaster_id={}",
            broadcaster_id,
        ))
        .header("Authorization", format!("Bearer {}", token.trim()))
        .header("Client-Id", client_id.trim())
        .send()
        .await?
        .json()
        .await?;

    let mut reward_id = String::new();
    for reward in res.data {
        if reward.title == "Highlight My Message (but without a message)" {
            reward_id = reward.id;
            break;
        }
    }

    if reward_id.len() == 0 {
        return Err("reward not found".into());
    };

    Ok(reward_id)
}
