use axum::{
    body::Bytes,
    extract::{
        ws::{Message, WebSocket},
        Path, Query, State, WebSocketUpgrade,
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use chrono::{Local, Utc};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, ClearType},
};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, VecDeque},
    io::{stdout, Write},
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

const GAME_VERSION: &str = "20231207";

const PHOTON_REALTIME: &str =
    "390700d6-7387-4fa5-b6d7-7ee39e46ad4a";

const PHOTON_VOICE: &str =
    "91c066ff-4be0-4c77-9733-d9e5831051a5";

const PHOTON_CHAT: &str =
    "5daaf075-d152-4c42-a4f7-47391cbff9be";

#[derive(Clone)]
struct AppState {
    started: Instant,
    requests: Arc<AtomicU64>,
    active_ws: Arc<AtomicU64>,
    logs: Arc<Mutex<VecDeque<String>>>,
    recent: Arc<Mutex<VecDeque<String>>>,
    tx: broadcast::Sender<String>,
    accounts: Arc<Mutex<HashMap<i64, Value>>>,
    running: Arc<AtomicBool>,
    port: u16,
}

impl AppState {
    async fn log(&self, msg: impl Into<String>) {
        let line = format!(
            "[{}] {}",
            Utc::now().format("%H:%M:%S%.3f"),
            msg.into()
        );

        let mut logs = self.logs.lock().unwrap();

        if logs.len() >= 500 {
            logs.pop_front();
        }

        logs.push_back(line.clone());
        drop(logs);

        let _ = self.tx.send(line);
    }
}

#[tokio::main]
async fn main() {
    /*
     * Deplexo fornece PORT.
     *
     * Local:
     *   PORT ausente -> 9000
     *
     * Deplexo:
     *   PORT=3000 -> 3000
     */
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(9000);

    let (tx, _) = broadcast::channel(512);

    let state = AppState {
        started: Instant::now(),

        requests: Arc::new(
            AtomicU64::new(0)
        ),

        active_ws: Arc::new(
            AtomicU64::new(0)
        ),

        logs: Arc::new(
            Mutex::new(VecDeque::new())
        ),

        recent: Arc::new(
            Mutex::new(VecDeque::new())
        ),

        tx,

        accounts: Arc::new(
            Mutex::new(HashMap::new())
        ),

        running: Arc::new(
            AtomicBool::new(true)
        ),

        port,
    };

    state
        .log("REC26 Rust Server starting")
        .await;

    let app = Router::new()

        // =====================================================
        // NS / DISCOVERY
        // =====================================================

        .route("/", get(ns_root))

        // =====================================================
        // VERSION / CONFIG
        // =====================================================

        .route(
            "/api/versioncheck/v4",
            get(version_check),
        )

        .route(
            "/api/gameconfigs/v1/all",
            get(empty_object),
        )

        .route(
            "/api/config/v1/amplitude",
            get(empty_object),
        )

        .route(
            "/api/config/v2",
            get(empty_object),
        )

        .route(
            "/api/config/v1/backtrace",
            get(empty_object),
        )

        .route(
            "/config/LoadingScreenTipData",
            get(empty_object),
        )

        // =====================================================
        // AVATAR
        // =====================================================

        .route(
            "/api/avatar/v1/defaultunlocked",
            get(empty_array),
        )

        .route(
            "/api/avatar/v1/defaultbaseavataritems",
            get(empty_array),
        )

        .route(
            "/api/avatar/v4/items",
            get(empty_array),
        )

        // =====================================================
        // OBJECTIVES / CHECKLIST
        // =====================================================

        .route(
            "/api/objectives/v1/myprogress",
            get(objectives),
        )

        .route(
            "/api/checklist/v1/current",
            get(empty_array),
        )

        // =====================================================
        // RELATIONSHIPS
        // =====================================================

        .route(
            "/api/relationships/v2/get",
            get(empty_array),
        )

        // =====================================================
        // QUICK PLAY
        // =====================================================

        .route(
            "/api/quickPlay/v1/getandclear",
            get(empty_object),
        )

        // =====================================================
        // PLAYER API
        // =====================================================

        .route(
            "/api/playerReputation/v2/bulk",
            get(reputation),
        )

        .route(
            "/api/players/v2/progression/bulk",
            get(progression),
        )

        // =====================================================
        // MODERATION
        // =====================================================

        .route(
            "/api/PlayerReporting/v1/moderationBlockDetails",
            post(empty_object_post),
        )

        // =====================================================
        // CUSTOM AVATAR
        // =====================================================

        .route(
            "/api/customAvatarItems/GetCustomAvatarItemCurrentSavesForLegacyAvatarItems",
            post(empty_array_post),
        )

        // =====================================================
        // AUTH
        // =====================================================

        .route(
            "/connect/token",
            post(connect_token),
        )

        // =====================================================
        // ACCOUNT
        // =====================================================

        .route(
            "/account/me",
            get(account_me),
        )

        .route(
            "/account/bulk",
            get(account_bulk),
        )

        // =====================================================
        // CACHED LOGIN
        // =====================================================

        .route(
            "/cachedlogin/forplatformids",
            post(empty_object_post),
        )

        .route(
            "/cachedlogin/forplatformid/:platform/:id",
            get(cached_login),
        )

        // =====================================================
        // PLAYER SETTINGS
        // =====================================================

        .route(
            "/playersettings",
            get(empty_object).put(ok_empty),
        )

        // =====================================================
        // OUTFITS
        // =====================================================

        .route(
            "/outfits/me",
            get(default_outfit),
        )

        // =====================================================
        // CRM
        // =====================================================

        .route(
            "/crm/me/config/v2",
            get(empty_object),
        )

        // =====================================================
        // STATSIG
        // =====================================================

        .route(
            "/statsigUserProperties",
            post(empty_object_post),
        )

        // =====================================================
        // TELEMETRY
        // =====================================================

        .route(
            "/data/event",
            post(ok_empty),
        )

        .route(
            "/data/heartbeat",
            post(ok_empty),
        )

        // =====================================================
        // EAC COMPATIBILITY ENDPOINT
        // =====================================================

        .route(
            "/eac/challenge",
            get(eac_challenge),
        )

        // =====================================================
        // MATCHMAKING PLAYER
        // =====================================================

        .route(
            "/player/qos",
            get(player_qos),
        )

        .route(
            "/player/connection-info",
            get(connection_info),
        )

        .route(
            "/player/login",
            post(player_login),
        )

        .route(
            "/player/loginlock",
            post(ok_empty),
        )

        .route(
            "/player/heartbeat",
            post(player_heartbeat),
        )

        .route(
            "/player/exclusivelogin",
            post(success_null),
        )

        .route(
            "/player/logout",
            post(ok_empty),
        )

        .route(
            "/player/photonregionpings",
            put(ok_empty),
        )

        .route(
            "/player",
            get(player_get),
        )

        // =====================================================
        // ROOMS
        // =====================================================

        .route(
            "/rooms",
            get(rooms),
        )

        // =====================================================
        // MATCHMAKING
        // =====================================================

        .route(
            "/matchmake/dorm",
            post(matchmake_dorm),
        )

        .route(
            "/matchmake/none",
            post(success_null),
        )

        .route(
            "/matchmake/v2/room/:id",
            post(matchmake_room),
        )

        .route(
            "/roominstance/:id/reportjoinresult",
            post(ok_empty),
        )

        // =====================================================
        // SIGNALR
        // =====================================================

        .route(
            "/hub/v1/negotiate",
            post(signalr_negotiate),
        )

        .route(
            "/hub/v1",
            get(signalr_ws),
        )

        // =====================================================
        // UNKNOWN ENDPOINT LOGGER
        // =====================================================

        .fallback(fallback)

        .layer(
            CorsLayer::permissive()
        )

        .with_state(
            state.clone()
        );

    // =========================================================
    // NETWORK
    // =========================================================

    let addr = SocketAddr::from(
        ([0, 0, 0, 0], port)
    );

    let listener =
        tokio::net::TcpListener::bind(addr)
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "Could not bind port {}: {}",
                    port,
                    e
                )
            });

    state
        .log(
            format!(
                "Listening on {} | build {}",
                addr,
                GAME_VERSION
            )
        )
        .await;

    // =========================================================
    // TUI
    // =========================================================

    let tui_state =
        state.clone();

    let tui =
        tokio::task::spawn_blocking(
            move || run_tui(tui_state)
        );

    // =========================================================
    // HTTP SERVER
    // =========================================================

    let server =
        axum::serve(
            listener,
            app
        )
        .with_graceful_shutdown(
            shutdown_signal(
                state.clone()
            )
        );

    if let Err(e) = server.await {
        state
            .log(
                format!(
                    "SERVER ERROR: {}",
                    e
                )
            )
            .await;
    }

    state
        .running
        .store(
            false,
            Ordering::Relaxed
        );

    let _ = tui.await;
}

// =============================================================
// SHUTDOWN
// =============================================================

async fn shutdown_signal(
    state: AppState
) {
    while state
        .running
        .load(Ordering::Relaxed)
    {
        tokio::time::sleep(
            Duration::from_millis(100)
        )
        .await;
    }
}

// =============================================================
// TERMINAL UI
// =============================================================

fn run_tui(
    state: AppState
) {
    let mut out =
        stdout();

    let _ =
        terminal::enable_raw_mode();

    let _ =
        execute!(
            out,
            terminal::EnterAlternateScreen,
            cursor::Hide
        );

    while state
        .running
        .load(Ordering::Relaxed)
    {
        if event::poll(
            Duration::from_millis(50)
        )
        .unwrap_or(false)
        {
            if let Ok(
                Event::Key(k)
            ) = event::read()
            {
                match k.code {
                    KeyCode::Char('q')
                    | KeyCode::Esc => {
                        state
                            .running
                            .store(
                                false,
                                Ordering::Relaxed
                            );

                        break;
                    }

                    KeyCode::Char('c') => {
                        state
                            .logs
                            .lock()
                            .unwrap()
                            .clear();
                    }

                    _ => {}
                }
            }
        }

        draw_tui(
            &state,
            &mut out
        );

        std::thread::sleep(
            Duration::from_millis(150)
        );
    }

    let _ =
        execute!(
            out,
            cursor::Show,
            terminal::LeaveAlternateScreen,
            ResetColor
        );

    let _ =
        terminal::disable_raw_mode();
}

fn draw_tui(
    state: &AppState,
    out: &mut std::io::Stdout,
) {
    let (w, h) =
        terminal::size()
            .unwrap_or(
                (120, 35)
            );

    let req =
        state.requests
            .load(Ordering::Relaxed);

    let ws =
        state.active_ws
            .load(Ordering::Relaxed);

    let accounts =
        state.accounts
            .lock()
            .unwrap()
            .len();

    let uptime =
        state.started
            .elapsed()
            .as_secs();

    let hh =
        uptime / 3600;

    let mm =
        (uptime % 3600) / 60;

    let ss =
        uptime % 60;

    let logs =
        state.logs
            .lock()
            .unwrap()
            .clone();

    let _ =
        execute!(
            out,
            cursor::MoveTo(0, 0),
            terminal::Clear(
                ClearType::All
            ),
            SetForegroundColor(
                Color::Cyan
            )
        );

    let title =
        " REC26 RUST SERVER 1.0  //  THE REC WE REMEMBER. ";

    let line =
        "═".repeat(
            w.saturating_sub(2)
                as usize
        );

    let _ =
        execute!(
            out,

            Print(
                format!(
                    "╔{}╗\r\n",
                    line
                )
            ),

            Print(
                format!(
                    "║{:<width$}║\r\n",
                    title,
                    width =
                        w.saturating_sub(2)
                            as usize
                )
            ),

            Print(
                format!(
                    "╠{}╣\r\n",
                    line
                )
            )
        );

    let _ =
        execute!(
            out,

            SetForegroundColor(
                Color::White
            ),

            Print(
                "  STATUS: "
            ),

            SetForegroundColor(
                Color::Green
            ),

            Print(
                "● ONLINE"
            ),

            SetForegroundColor(
                Color::White
            ),

            Print(
                format!(
                    "    BUILD: {}    ADDRESS: 0.0.0.0:{}    UPTIME: {:02}:{:02}:{:02}\r\n",
                    GAME_VERSION,
                    state.port,
                    hh,
                    mm,
                    ss
                )
            )
        );

    let _ =
        execute!(
            out,

            Print(
                format!(
                    "  REQUESTS: {:<8}  WEBSOCKETS: {:<4}  KNOWN ACCOUNTS: {:<4}  TIME: {}\r\n",
                    req,
                    ws,
                    accounts,
                    Local::now()
                        .format("%H:%M:%S")
                )
            )
        );

    let _ =
        execute!(
            out,

            SetForegroundColor(
                Color::Cyan
            ),

            Print(
                format!(
                    "╠{}╣\r\n",
                    line
                )
            ),

            Print(
                "║ SERVICES                                                                                                                     ║\r\n"
            ),

            SetForegroundColor(
                Color::Green
            ),

            Print(
                "  ● API      ● MATCH      ● ROOMS      ● NOTIFICATIONS"
            ),

            SetForegroundColor(
                Color::Yellow
            ),

            Print(
                "      ◐ CHAT      ◐ CLUBS\r\n"
            )
        );

    let _ =
        execute!(
            out,

            SetForegroundColor(
                Color::Cyan
            ),

            Print(
                format!(
                    "╠{}╣\r\n",
                    line
                )
            ),

            Print(
                "║ LIVE SERVER LOGS                                                                                                             ║\r\n"
            ),

            SetForegroundColor(
                Color::White
            )
        );

    let used =
        9u16;

    let rows =
        h.saturating_sub(
            used + 3
        ) as usize;

    let start =
        logs.len()
            .saturating_sub(
                rows
            );

    for l in
        logs.iter()
            .skip(start)
    {
        let mut s =
            l.clone();

        let max =
            w.saturating_sub(4)
                as usize;

        if s.len() > max {
            s.truncate(max);
        }

        let color =
            if s.contains("404")
                || s.contains("ERROR")
            {
                Color::Red
            } else if s.contains("[WS]") {
                Color::Magenta
            } else if s.contains("[MATCH]") {
                Color::Yellow
            } else {
                Color::White
            };

        let _ =
            execute!(
                out,

                SetForegroundColor(
                    color
                ),

                Print(
                    format!(
                        "  {}\r\n",
                        s
                    )
                )
            );
    }

    let _ =
        execute!(
            out,

            SetForegroundColor(
                Color::Cyan
            ),

            cursor::MoveTo(
                0,
                h.saturating_sub(2)
            ),

            Print(
                format!(
                    "╚{}╝",
                    line
                )
            ),

            cursor::MoveTo(
                0,
                h.saturating_sub(1)
            ),

            SetForegroundColor(
                Color::Yellow
            ),

            Print(
                " [Q/Esc] Quit   [C] Clear logs "
            ),

            ResetColor
        );

    let _ =
        out.flush();
}

// =============================================================
// REQUEST LOGGING
// =============================================================

async fn hit(
    s: &AppState,
    msg: &str,
) {
    s.requests
        .fetch_add(
            1,
            Ordering::Relaxed
        );

    {
        let mut r =
            s.recent
                .lock()
                .unwrap();

        if r.len() >= 64 {
            r.pop_front();
        }

        r.push_back(
            msg.to_string()
        );
    }

    s.log(
        format!(
            "[HTTP] {}",
            msg
        )
    )
    .await;
}

// =============================================================
// NS
// =============================================================

async fn ns_root(
    State(s): State<AppState>,
    headers: HeaderMap,
) -> Json<Value> {
    let host = headers
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("?")
        .split(':')
        .next()
        .unwrap_or("?")
        .to_ascii_lowercase();

    hit(&s, &format!("GET / host={}", host)).await;

    // Public REC26 service discovery. All hostnames may terminate at the
    // same Deplexo deployment; the Host header identifies the service.
    Json(json!({
        "Accounts": "https://accounts.rec26.dedyn.io",
        "AI": "https://ai.rec26.dedyn.io",
        "API": "https://api.rec26.dedyn.io",
        "Auth": "https://auth.rec26.dedyn.io",
        "BugReporting": "https://bugreporting.rec26.dedyn.io",
        "Cards": "https://cards.rec26.dedyn.io",
        "CDN": "https://cdn.rec26.dedyn.io",
        "Chat": "https://chat.rec26.dedyn.io",
        "Clubs": "https://clubs.rec26.dedyn.io",
        "CMS": "https://cms.rec26.dedyn.io",
        "Commerce": "https://commerce.rec26.dedyn.io",
        "Data": "https://data.rec26.dedyn.io",
        "DataCollection": "https://datacollection.rec26.dedyn.io",
        "Discovery": "https://discovery.rec26.dedyn.io",
        "Econ": "https://econ.rec26.dedyn.io",
        "GameLogs": "https://gamelogs.rec26.dedyn.io",
        "Geo": "https://geo.rec26.dedyn.io",
        "Images": "https://img.rec26.dedyn.io",
        "Leaderboards": "https://leaderboard.rec26.dedyn.io",
        "Link": "https://link.rec26.dedyn.io",
        "Lists": "https://lists.rec26.dedyn.io",
        "Matchmaking": "https://match.rec26.dedyn.io",
        "Moderation": "https://moderation.rec26.dedyn.io",
        "Notifications": "https://notify.rec26.dedyn.io",
        "PlatformNotifications": "https://platformnotifications.rec26.dedyn.io",
        "PlayerSettings": "https://playersettings.rec26.dedyn.io",
        "RoomComments": "https://roomcomments.rec26.dedyn.io",
        "RoomieIntegrations": "https://roomieintegrations.rec26.dedyn.io",
        "Rooms": "https://rooms.rec26.dedyn.io",
        "Storage": "https://storage.rec26.dedyn.io",
        "Strings": "https://strings.rec26.dedyn.io",
        "StringsCDN": "https://strings-cdn.rec26.dedyn.io",
        "Studio": "https://studio.rec26.dedyn.io",
        "Thorn": "https://thorn.rec26.dedyn.io",
        "Videos": "https://videos.rec26.dedyn.io",
        "WWW": "https://www.rec26.dedyn.io"
    }))
}

// =============================================================
// GENERIC RESPONSES
// =============================================================

async fn empty_object(
    State(s): State<AppState>,
) -> Json<Value> {
    hit(
        &s,
        "compat -> {}"
    )
    .await;

    Json(
        json!({})
    )
}

async fn empty_array(
    State(s): State<AppState>,
) -> Json<Value> {
    hit(
        &s,
        "compat -> []"
    )
    .await;

    Json(
        json!([])
    )
}

async fn empty_object_post(
    State(s): State<AppState>,
    _b: Bytes,
) -> Json<Value> {
    hit(
        &s,
        "POST compat -> {}"
    )
    .await;

    Json(
        json!({})
    )
}

async fn empty_array_post(
    State(s): State<AppState>,
    _b: Bytes,
) -> Json<Value> {
    hit(
        &s,
        "POST compat -> []"
    )
    .await;

    Json(
        json!([])
    )
}

async fn ok_empty(
    State(s): State<AppState>,
    _b: Bytes,
) -> StatusCode {
    hit(
        &s,
        "compat -> 200"
    )
    .await;

    StatusCode::OK
}

async fn success_null(
    State(s): State<AppState>,
    _b: Bytes,
) -> Json<Value> {
    hit(
        &s,
        "success/null"
    )
    .await;

    Json(
        json!({
            "success": true,
            "error": null,
            "value": null
        })
    )
}

// =============================================================
// VERSION
// =============================================================

async fn version_check(
    State(s): State<AppState>,
    Query(q): Query<HashMap<String, String>>,
) -> Json<Value> {
    hit(
        &s,
        &format!(
            "GET /api/versioncheck/v4 {:?}",
            q
        )
    )
    .await;

    Json(
        json!({
            "VersionStatus": 0,
            "UpdateNotificationStage": 0,
            "IsVersionIslanded": false,
            "IsCrossPlayDisabled": false
        })
    )
}

// =============================================================
// OBJECTIVES
// =============================================================

async fn objectives(
    State(s): State<AppState>,
) -> Json<Value> {
    hit(
        &s,
        "GET objectives"
    )
    .await;

    Json(
        json!({
            "Objectives": [],
            "ObjectiveGroups": []
        })
    )
}

// =============================================================
// REPUTATION
// =============================================================

async fn reputation(
    State(s): State<AppState>,
) -> Json<Value> {
    hit(
        &s,
        "GET reputation"
    )
    .await;

    Json(
        json!([])
    )
}

// =============================================================
// PROGRESSION
// =============================================================

async fn progression(
    State(s): State<AppState>,
    Query(q): Query<HashMap<String, String>>,
) -> Json<Value> {
    hit(
        &s,
        "GET progression"
    )
    .await;

    let id =
        q.get("id")
            .and_then(
                |x| x.parse::<i64>().ok()
            )
            .unwrap_or(0);

    Json(
        json!([
            {
                "PlayerId": id,
                "Level": 1,
                "XP": 0
            }
        ])
    )
}

// =============================================================
// AUTH
// =============================================================

async fn connect_token(
    State(s): State<AppState>,
    body: Bytes,
) -> Json<Value> {
    hit(
        &s,
        "POST /connect/token"
    )
    .await;

    let raw =
        String::from_utf8_lossy(
            &body
        );

    s.log(
        format!(
            "[AUTH] {}",
            raw
        )
    )
    .await;

    let account_id =
        form_value(
            &raw,
            "account_id"
        )
        .and_then(
            |v| v.parse::<i64>().ok()
        )
        .unwrap_or(1);

    let acct =
        json!({
            "accountId": account_id,
            "username": format!("REC26_{}", account_id),
            "displayName": format!("REC26_{}", account_id)
        });

    s.accounts
        .lock()
        .unwrap()
        .insert(
            account_id,
            acct
        );

    Json(
        json!({
            "access_token":
                format!(
                    "rec26-{}-{}",
                    account_id,
                    Uuid::new_v4()
                ),

            "token_type": "Bearer",
            "expires_in": 3600,

            "refresh_token":
                Uuid::new_v4()
                    .to_string(),

            "accountId":
                account_id
        })
    )
}

fn form_value(
    raw: &str,
    key: &str,
) -> Option<String> {
    raw.split('&')
        .find_map(
            |p| {
                let mut it =
                    p.splitn(
                        2,
                        '='
                    );

                if it.next()? == key {
                    Some(
                        it.next()
                            .unwrap_or("")
                            .to_string()
                    )
                } else {
                    None
                }
            }
        )
}

// =============================================================
// ACCOUNTS
// =============================================================

async fn account_me(
    State(s): State<AppState>,
) -> Json<Value> {
    hit(
        &s,
        "GET /account/me"
    )
    .await;

    let a =
        s.accounts
            .lock()
            .unwrap()
            .values()
            .next()
            .cloned()
            .unwrap_or(
                json!({
                    "accountId": 1,
                    "username": "REC26",
                    "displayName": "REC26"
                })
            );

    Json(a)
}

async fn account_bulk(
    State(s): State<AppState>,
    Query(q): Query<HashMap<String, String>>,
) -> Json<Value> {
    hit(
        &s,
        "GET /account/bulk"
    )
    .await;

    let id =
        q.get("id")
            .and_then(
                |x| x.parse::<i64>().ok()
            )
            .unwrap_or(1);

    Json(
        json!([
            {
                "accountId": id,
                "username": format!("REC26_{}", id),
                "displayName": format!("REC26_{}", id)
            }
        ])
    )
}

// =============================================================
// CACHED LOGIN
// =============================================================

async fn cached_login(
    State(s): State<AppState>,
    Path((_platform, id)):
        Path<(String, String)>,
) -> Json<Value> {
    hit(
        &s,
        "GET cachedlogin"
    )
    .await;

    Json(
        json!({
            "accountId":
                id.parse::<i64>()
                    .unwrap_or(1)
        })
    )
}

// =============================================================
// OUTFIT
// =============================================================

async fn default_outfit(
    State(s): State<AppState>,
) -> Json<Value> {
    hit(
        &s,
        "GET /outfits/me"
    )
    .await;

    Json(
        json!({})
    )
}

// =============================================================
// EAC ENDPOINT COMPATIBILITY
// =============================================================

async fn eac_challenge(
    State(s): State<AppState>,
) -> Json<Value> {
    hit(
        &s,
        "GET /eac/challenge"
    )
    .await;

    Json(
        json!({})
    )
}

// =============================================================
// QOS
// =============================================================

async fn player_qos(
    State(s): State<AppState>,
) -> Json<Value> {
    hit(
        &s,
        "GET /player/qos"
    )
    .await;

    Json(
        json!([
            {
                "id": "us-east1",
                "address": "127.0.0.1:5055"
            }
        ])
    )
}

// =============================================================
// CONNECTION INFO
// =============================================================

async fn connection_info(
    State(s): State<AppState>,
) -> Json<Value> {
    hit(
        &s,
        "GET /player/connection-info"
    )
    .await;

    Json(
        json!({
            "success": true,
            "error": null,

            "value": {
                "experiments": {},

                "photonAuthToken": null,

                "photonChatAppId":
                    PHOTON_CHAT,

                "photonRealtimeAppId":
                    PHOTON_REALTIME,

                "photonRegion":
                    "us",

                "photonRoomId":
                    "",

                "photonVoiceAppId":
                    PHOTON_VOICE,

                "voiceConnectionInfo":
                    "",

                "voiceServerId":
                    ""
            }
        })
    )
}

// =============================================================
// PLAYER LOGIN
// =============================================================

async fn player_login(
    State(s): State<AppState>,
) -> Json<Value> {
    hit(
        &s,
        "POST /player/login"
    )
    .await;

    Json(
        json!({
            "success": true,
            "error": null,

            "value": {
                "sessionToken":
                    Uuid::new_v4()
                        .to_string(),

                "playerId":
                    1
            }
        })
    )
}

// =============================================================
// HEARTBEAT
// =============================================================

async fn player_heartbeat(
    State(s): State<AppState>,
) -> Json<Value> {
    hit(
        &s,
        "POST /player/heartbeat"
    )
    .await;

    Json(
        json!({
            "appVersion":
                GAME_VERSION,

            "deviceClass":
                2,

            "errorCode":
                0,

            "experiments":
                null,

            "isOnline":
                true,

            "photonAuthToken":
                null,

            "photonChatAppId":
                PHOTON_CHAT,

            "photonRealtimeAppId":
                PHOTON_REALTIME,

            "photonRegion":
                "us",

            "photonRoomId":
                null,

            "photonVoiceAppId":
                PHOTON_VOICE,

            "platform":
                0,

            "playerId":
                1,

            "roomInstance":
                null,

            "statusVisibility":
                0,

            "voiceConnectionInfo":
                null,

            "voiceServerId":
                null,

            "vrMovementMode":
                0
        })
    )
}

// =============================================================
// GET PLAYER
// =============================================================

async fn player_get(
    State(s): State<AppState>,
    Query(q): Query<HashMap<String, String>>,
) -> Json<Value> {
    hit(
        &s,
        "GET /player"
    )
    .await;

    let id =
        q.get("id")
            .and_then(
                |x| x.parse::<i64>().ok()
            )
            .unwrap_or(1);

    Json(
        json!({
            "playerId": id,
            "isOnline": true,
            "statusVisibility": 0,
            "roomInstance": null
        })
    )
}

// =============================================================
// ROOMS
// =============================================================

async fn rooms(
    State(s): State<AppState>,
    Query(q): Query<HashMap<String, String>>,
) -> Json<Value> {
    hit(
        &s,
        &format!(
            "GET /rooms {:?}",
            q
        )
    )
    .await;

    let name =
        q.get("name")
            .cloned()
            .unwrap_or_default();

    if name
        .eq_ignore_ascii_case(
            "Orientation"
        )
    {
        Json(
            json!({
                "Name":
                    "Orientation"
            })
        )
    } else {
        Json(
            json!({})
        )
    }
}

// =============================================================
// DORM MATCHMAKING
// =============================================================

async fn matchmake_dorm(
    State(s): State<AppState>,
    body: Bytes,
) -> Json<Value> {
    hit(
        &s,
        "POST /matchmake/dorm"
    )
    .await;

    s.log(
        format!(
            "[MATCH] dorm body={}",
            String::from_utf8_lossy(
                &body
            )
        )
    )
    .await;

    /*
     * Compatibility probe.
     * Exact historical RoomInstance DTO
     * is still being reconstructed.
     */
    Json(
        json!({
            "success": true,
            "error": null,

            "value": {
                "RoomInstance": {
                    "RoomInstanceId": 1,
                    "RoomInstanceType": 0
                }
            }
        })
    )
}

// =============================================================
// ROOM MATCHMAKING
// =============================================================

async fn matchmake_room(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> Json<Value> {
    hit(
        &s,
        &format!(
            "POST /matchmake/v2/room/{}",
            id
        )
    )
    .await;

    Json(
        json!({
            "success": true,
            "error": null,

            "value": {
                "RoomInstance": {
                    "RoomInstanceId": 1,
                    "RoomInstanceType": 0
                }
            }
        })
    )
}

// =============================================================
// SIGNALR NEGOTIATE
// =============================================================

async fn signalr_negotiate(
    State(s): State<AppState>,
) -> Json<Value> {
    hit(
        &s,
        "POST /hub/v1/negotiate"
    )
    .await;

    let id =
        Uuid::new_v4()
            .simple()
            .to_string();

    Json(
        json!({
            "connectionId":
                id,

            "connectionToken":
                id,

            "negotiateVersion":
                1,

            "availableTransports": [
                {
                    "transport":
                        "WebSockets",

                    "transferFormats": [
                        "Text",
                        "Binary"
                    ]
                }
            ]
        })
    )
}

// =============================================================
// SIGNALR WEBSOCKET
// =============================================================

async fn signalr_ws(
    State(s): State<AppState>,
    ws: WebSocketUpgrade,
) -> Response {
    hit(
        &s,
        "WS /hub/v1"
    )
    .await;

    ws.on_upgrade(
        move |sock| {
            notify_socket(
                sock,
                s
            )
        }
    )
}

async fn notify_socket(
    mut sock: WebSocket,
    s: AppState,
) {
    s.active_ws
        .fetch_add(
            1,
            Ordering::Relaxed
        );

    s.log(
        "[WS] notification connected"
    )
    .await;

    while let Some(
        Ok(msg)
    ) = sock.recv().await
    {
        match msg {
            Message::Text(t) => {
                if t.contains(
                    "protocol"
                ) {
                    let _ =
                        sock.send(
                            Message::Text(
                                "{}\u{1e}".into()
                            )
                        )
                        .await;
                } else if t.contains(
                    "invocationId"
                ) {
                    let _ =
                        sock.send(
                            Message::Text(
                                "{\"type\":3,\"invocationId\":\"1\",\"result\":null}\u{1e}"
                                    .into()
                            )
                        )
                        .await;
                }
            }

            Message::Binary(b) => {
                let t =
                    String::from_utf8_lossy(
                        &b
                    );

                if t.contains(
                    "protocol"
                ) {
                    let _ =
                        sock.send(
                            Message::Binary(
                                b"{}\x1e".to_vec()
                            )
                        )
                        .await;
                } else if t.contains(
                    "invocationId"
                ) {
                    let _ =
                        sock.send(
                            Message::Binary(
                                b"{\"type\":3,\"invocationId\":\"1\",\"result\":null}\x1e"
                                    .to_vec()
                            )
                        )
                        .await;
                }
            }

            Message::Close(_) => {
                break;
            }

            _ => {}
        }
    }

    s.active_ws
        .fetch_sub(
            1,
            Ordering::Relaxed
        );

    s.log(
        "[WS] notification disconnected"
    )
    .await;
}

// =============================================================
// FALLBACK
// =============================================================

async fn fallback(
    State(s): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let host =
        headers
            .get("host")
            .and_then(
                |v| v.to_str().ok()
            )
            .unwrap_or("?");

    hit(
        &s,
        &format!(
            "[404] host={}",
            host
        )
    )
    .await;

    (
        StatusCode::NOT_FOUND,

        Json(
            json!({
                "error":
                    "REC26 route not implemented"
            })
        ),
    )
}
