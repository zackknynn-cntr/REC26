mod domains;

use axum::{
    extract::{Path, Query, State, WebSocketUpgrade, ws::{Message, WebSocket}},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, env, net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{info, warn};
use uuid::Uuid;

const GAME_VERSION: &str = "20250718.01";
const MANIFEST: &str = "1151455856673601091";

#[derive(Clone, Default)]
struct AppState {
    players: Arc<RwLock<HashMap<i64, Player>>>,
    settings: Arc<RwLock<HashMap<i64, Value>>>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Player {
    #[serde(rename="Id")] id: i64,
    #[serde(rename="Username")] username: String,
    #[serde(rename="DisplayName")] display_name: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter(
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into())
    ).init();

    let state = AppState::default();
    let app = Router::new()
        .route("/", get(ns_root))
        .route("/__health", get(health))
        .route("/api/versioncheck/v4", get(versioncheck))
        .route("/api/gameconfigs/v1/all", get(empty_object))
        .route("/api/config/v1/amplitude", get(empty_object))
        .route("/api/config/v2", get(empty_object))
        .route("/api/checklist/v1/current", get(empty_object))
        .route("/api/objectives/v1/myprogress", get(empty_array))
        .route("/api/avatar/v1/defaultunlocked", get(empty_array))
        .route("/api/avatar/v1/defaultbaseavataritems", get(empty_array))
        .route("/api/avatar/v4/items", get(empty_array))
        .route("/api/relationships/v2/get", get(empty_array))
        .route("/api/players/v2/progression/bulk", get(empty_array))
        .route("/api/playerReputation/v2/bulk", get(empty_array))
        .route("/api/customAvatarItems/v1/isCreationAllowedForAccount", get(|| async { Json(json!(false)) }))
        .route("/api/customAvatarItems/GetCustomAvatarItemCurrentSavesForLegacyAvatarItems", post(empty_array))
        .route("/api/PlayerReporting/v1/moderationBlockDetails", post(empty_object))
        .route("/api/storefronts/v3/giftdropstore/{id}", get(storefront))
        .route("/connect/token", post(connect_token))
        .route("/eac/challenge", get(eac_challenge_placeholder))
        .route("/account/me", get(account_me))
        .route("/account/bulk", get(account_bulk))
        .route("/player", get(player_get))
        .route("/player/login", post(player_login))
        .route("/player/heartbeat", post(player_heartbeat))
        .route("/player/qos", get(empty_array))
        .route("/player/connection-info", get(connection_info))
        .route("/player/exclusivelogin", post(success_null))
        .route("/matchmake/dorm", post(matchmake_dorm))
        .route("/rooms", get(rooms_query))
        .route("/outfits/me", get(empty_object))
        .route("/econ/customAvatarItems/v1/owned", get(empty_array))
        .route("/playersettings", get(settings_get).put(settings_put))
        .route("/crm/me/config/v2", get(empty_object))
        .route("/data/heartbeat", post(empty_object))
        .route("/data/event", post(empty_object))
        .route("/statsigUserProperties", post(empty_object))
        .route("/cachedlogin/forplatformids", post(empty_array))
        .route("/cachedlogin/forplatformid/{platform}/{platform_id}", get(cached_login))
        .route("/hub/v1/negotiate", post(signalr_negotiate))
        .route("/hub/v1", get(signalr_ws))
        .fallback(fallback)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let port: u16 = env::var("PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(3000);
    let addr = SocketAddr::from(([0,0,0,0], port));
    info!("REC26 20250718.01 listening on {addr}; manifest={MANIFEST}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}


async fn health() -> Json<Value> { Json(json!({"ok":true,"version":GAME_VERSION,"manifest":MANIFEST})) }
async fn empty_object() -> Json<Value> { Json(json!({})) }
async fn empty_array() -> Json<Value> { Json(json!([])) }
async fn success_null() -> Json<Value> { Json(json!({"success":true,"error":null,"value":null})) }

async fn ns_root(headers: HeaderMap) -> Json<Value> {
    let host = headers.get("host").and_then(|v| v.to_str().ok()).unwrap_or("");
    info!("[{}] service discovery host={host}", domains::area_for_host(host).label());

    // Keep the full service-discovery contract while using only the six
    // REC26 hostnames that are currently deployed. Services without a
    // dedicated public hostname are handled by the API area for now.
    Json(json!({
        "Accounts": domains::service_url("Accounts"),
        "AI": domains::service_url("AI"),
        "API": domains::service_url("API"),
        "Auth": domains::service_url("Auth"),
        "BugReporting": domains::service_url("BugReporting"),
        "Cards": domains::service_url("Cards"),
        "CDN": domains::service_url("CDN"),
        "Chat": domains::service_url("Chat"),
        "Clubs": domains::service_url("Clubs"),
        "CMS": domains::service_url("CMS"),
        "Commerce": domains::service_url("Commerce"),
        "Data": domains::service_url("Data"),
        "DataCollection": domains::service_url("DataCollection"),
        "Discovery": domains::service_url("Discovery"),
        "Econ": domains::service_url("Econ"),
        "GameLogs": domains::service_url("GameLogs"),
        "Geo": domains::service_url("Geo"),
        "Images": domains::service_url("Images"),
        "Leaderboard": domains::service_url("Leaderboard"),
        "Link": domains::service_url("Link"),
        "Lists": domains::service_url("Lists"),
        "Matchmaking": domains::service_url("Matchmaking"),
        "Moderation": domains::service_url("Moderation"),
        "Notifications": domains::service_url("Notifications"),
        "PlatformNotifications": domains::service_url("PlatformNotifications"),
        "PlayerSettings": domains::service_url("PlayerSettings"),
        "RoomComments": domains::service_url("RoomComments"),
        "RoomieIntegrations": domains::service_url("RoomieIntegrations"),
        "Rooms": domains::service_url("Rooms"),
        "Storage": domains::service_url("Storage"),
        "Strings": domains::service_url("Strings"),
        "StringsCDN": domains::service_url("StringsCDN"),
        "Studio": domains::service_url("Studio"),
        "Thorn": domains::service_url("Thorn"),
        "Videos": domains::service_url("Videos"),
        "WWW": domains::service_url("WWW")
    }))
}

#[derive(Deserialize)] struct VersionQ { v: Option<String>, p: Option<i32>, pid: Option<i64> }
async fn versioncheck(Query(q): Query<VersionQ>) -> Json<Value> {
    info!("versioncheck v={:?} p={:?} pid={:?}", q.v,q.p,q.pid);
    Json(json!({"VersionStatus":0,"UpdateNotificationStage":0,"IsVersionIslanded":false,"IsCrossPlayDisabled":false}))
}

async fn connect_token(headers: HeaderMap, body: String) -> Json<Value> {
    info!("connect/token content-type={:?} bytes={}", headers.get("content-type"), body.len());
    Json(json!({"access_token":Uuid::new_v4().to_string(),"token_type":"Bearer","expires_in":86400,"refresh_token":Uuid::new_v4().to_string()}))
}

// This is intentionally a compatibility placeholder, not an anti-cheat bypass.
async fn eac_challenge_placeholder() -> Response {
    (StatusCode::NOT_IMPLEMENTED, Json(json!({"error":"EAC challenge not implemented by REC26"}))).into_response()
}

async fn account_me(State(st): State<AppState>) -> Json<Value> {
    let id=1i64; let mut p=st.players.write().await;
    let pl=p.entry(id).or_insert(Player{id,username:"REC26Player".into(),display_name:"REC26 Player".into()}).clone();
    Json(serde_json::to_value(pl).unwrap())
}
async fn account_bulk(State(st): State<AppState>) -> Json<Value> {
    let p=st.players.read().await; Json(json!(p.values().cloned().collect::<Vec<_>>()))
}
#[derive(Deserialize)] struct PlayerQ { id: Option<i64> }
async fn player_get(State(st): State<AppState>, Query(q): Query<PlayerQ>) -> Json<Value> {
    let id=q.id.unwrap_or(1); let mut p=st.players.write().await;
    let pl=p.entry(id).or_insert(Player{id,username:format!("Player{id}"),display_name:format!("Player{id}")}).clone();
    Json(serde_json::to_value(pl).unwrap())
}
async fn player_login(State(st): State<AppState>, body: String) -> Json<Value> {
    info!("player/login bytes={}",body.len()); let id=1i64;
    st.players.write().await.entry(id).or_insert(Player{id,username:"REC26Player".into(),display_name:"REC26 Player".into()});
    Json(json!({"success":true,"error":null,"value":{"PlayerId":id}}))
}
async fn player_heartbeat() -> Json<Value> { Json(json!({"success":true,"error":null,"value":null})) }
async fn connection_info() -> Json<Value> {
    Json(json!({"success":true,"error":null,"value":{"PhotonRealtimeAppId":env::var("PHOTON_REALTIME_APP_ID").unwrap_or_default(),"PhotonVoiceAppId":env::var("PHOTON_VOICE_APP_ID").unwrap_or_default(),"PhotonChatAppId":env::var("PHOTON_CHAT_APP_ID").unwrap_or_default(),"PhotonRegion":"us","PhotonAuthToken":Value::Null}}))
}
async fn matchmake_dorm() -> Json<Value> { Json(json!({"success":true,"error":null,"value":{"RoomInstance":{"RoomInstanceId":1,"RoomInstanceType":0}}})) }
#[derive(Deserialize)] struct RoomQ { name: Option<String> }
async fn rooms_query(Query(q): Query<RoomQ>) -> Json<Value> { Json(json!({"Name":q.name.unwrap_or_else(||"DormRoom".into())})) }
async fn storefront(Path(id): Path<i64>) -> Json<Value> { Json(json!({"Id":id,"Items":[]})) }
async fn settings_get(State(st): State<AppState>) -> Json<Value> { Json(st.settings.read().await.get(&1).cloned().unwrap_or_else(||json!({}))) }
async fn settings_put(State(st): State<AppState>, Json(v): Json<Value>) -> Json<Value> { st.settings.write().await.insert(1,v); Json(json!({})) }
async fn cached_login(Path((_platform,_platform_id)): Path<(i32,String)>) -> Json<Value> { Json(json!({"AccountId":1})) }

async fn signalr_negotiate() -> Json<Value> {
    let id=Uuid::new_v4().simple().to_string();
    Json(json!({"connectionId":id,"connectionToken":id,"negotiateVersion":1,"availableTransports":[{"transport":"WebSockets","transferFormats":["Text","Binary"]}]}))
}
async fn signalr_ws(ws: WebSocketUpgrade) -> impl IntoResponse { ws.on_upgrade(handle_ws) }
async fn handle_ws(mut socket: WebSocket) {
    while let Some(Ok(msg))=socket.recv().await {
        match msg {
            Message::Text(t) => { let _=socket.send(Message::Text(signalr_reply(&t).into())).await; },
            Message::Binary(b) => { let s=String::from_utf8_lossy(&b); let _=socket.send(Message::Text(signalr_reply(&s).into())).await; },
            Message::Ping(p) => { let _=socket.send(Message::Pong(p)).await; },
            Message::Close(_) => break,
            _=>{}
        }
    }
}
fn signalr_reply(s:&str)->String {
    let rs='\u{001e}';
    if s.contains("\"protocol\"") { return format!("{{}}{rs}"); }
    if s.contains("SubscribeToPlayers") {
        let id=serde_json::from_str::<Value>(s.trim_end_matches(rs)).ok().and_then(|v|v.get("invocationId").and_then(Value::as_str).map(str::to_string)).unwrap_or_else(||"1".into());
        return format!("{{\"type\":3,\"invocationId\":\"{}\",\"result\":null}}{rs}",id);
    }
    format!("{{\"type\":6}}{rs}")
}

async fn fallback(headers: HeaderMap, req: axum::extract::Request) -> Response {
    let host=headers.get("host").and_then(|v|v.to_str().ok()).unwrap_or("");
    warn!("[{}] 404 host={} method={} path={}",domains::area_for_host(host).label(),host,req.method(),req.uri());
    (StatusCode::NOT_FOUND, Json(json!({"error":"REC26 route not implemented","host":host,"path":req.uri().path(),"targetBuild":GAME_VERSION}))).into_response()
}
