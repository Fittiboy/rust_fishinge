use serde::{Deserialize, Serialize};
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

pub async fn get_ids() -> Result<(String, String), Box<dyn std::error::Error>> {
    let streamer = read_to_string("./streamer.txt").expect("needs to have streamer.txt present");
    let token = read_to_string("./uat.txt").expect("needs to have uat.txt present");
    let client_id = read_to_string("./client_id.txt").expect("needs to have client_id.txt present");
    let reward_title =
        read_to_string("./reward_title.txt").expect("needs to have reward_title.txt present");
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
        if reward.title == reward_title.trim() {
            reward_id = reward.id;
            break;
        }
    }

    if reward_id.len() == 0 {
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
    session_id: String,
    broadcaster_id: String,
    reward_id: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let token = read_to_string("./uat.txt").expect("needs to have uat.txt present");
    let client_id = read_to_string("./client_id.txt").expect("needs to have client_id.txt present");
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

    let res = client
        .post("https://api.twitch.tv/helix/eventsub/subscriptions")
        .header("Authorization", format!("Bearer {}", token.trim()))
        .header("Client-Id", client_id.trim())
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?
        .json::<SubscriptionResponse>()
        .await?;

    println!("{:#?}", res);
    Ok(())
}
