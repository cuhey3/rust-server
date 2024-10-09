use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    env,
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast;
use tokio::sync::broadcast::Sender;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Our shared state
struct AppState {
    // Channel used to send messages to all connected clients.
    tx: Sender<String>,
    channel_list: Mutex<Vec<String>>,
    channel_tx_list: Mutex<Vec<Sender<String>>>,
}

#[derive(Deserialize, Debug)]
struct ChannelUser {
    user_name: String,
    channel_name: String,
}

#[derive(Serialize)]
struct ChannelMessage {
    user_name: String,
    message_type: MessageType,
    message: String,
}

#[derive(Serialize)]
enum MessageType {
    Message,
    Join,
    Left,
}
#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "example_chat=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Set up application state for use with with_state().
    let channel_list: Mutex<Vec<String>> = Mutex::new(vec![]);
    let (tx, _rx) = broadcast::channel(100);
    let channel_tx_list = Mutex::new(vec![tx.clone()]);

    let app_state = Arc::new(AppState {
        tx,
        channel_list,
        channel_tx_list,
    });

    let app = Router::new()
        .route("/", get(index))
        .route("/ws", get(websocket_handler))
        .with_state(app_state);
    let mut port: u16 = 8080;
    match env::var("PORT") {
        Ok(p) => {
            match p.parse::<u16>() {
                Ok(n) => {
                    port = n;
                }
                Err(_e) => {}
            };
        }
        Err(_e) => {}
    };
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(socket, state))
}

// This function deals with a single websocket connection, i.e., a single
// connected client / user, for which we will spawn two independent tasks (for
// receiving / sending chat messages).
async fn websocket(stream: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = stream.split();

    // Username gets set in the receive loop, if it's valid.
    let mut channel_user = ChannelUser {
        user_name: "".to_string(),
        channel_name: "".to_string(),
    };
    let mut channel_index = 0_usize;
    let (_tx, _rx) = broadcast::channel(100);
    let mut state_tx: Sender<String> = _tx;
    while let Some(Ok(message)) = receiver.next().await {
        if let Message::Text(name) = message {
            let requested: Result<ChannelUser, _> = serde_json::from_str(&*name);
            if requested.is_ok() {
                channel_user = requested.unwrap();
                state_tx = check_channel(&state, &channel_user.channel_name);
                break;
            } else {
                // Only send our client that request is invalid.
                let _ = sender
                    .send(Message::Text(String::from("Invalid request. e.g. {\"user_name\": \"name\", \"channel_name\": \"name\"}")))
                    .await;

                return;
            }
        }
    }
    // We subscribe *before* sending the "joined" message, so that we will also
    // display it to our client.
    let mut rx = state_tx.subscribe();

    // Now send the "joined" message to all subscribers.
    let channel_message = ChannelMessage {
        user_name: channel_user.user_name.to_owned(),
        message_type: MessageType::Join,
        message: format!("{} joined.", channel_user.user_name.to_owned()),
    };
    let msg = serde_json::to_string(&channel_message).unwrap();
    tracing::debug!("{msg}");
    let _ = state_tx.send(msg);

    // Spawn the first task that will receive broadcast messages and send text
    // messages over the websocket to our client.
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            // In any websocket error, break loop.
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });
    // Clone things we want to pass (move) to the receiving task.
    let tx = state_tx.clone();
    let name = channel_user.user_name.clone();

    // Spawn a task that takes messages from the websocket, prepends the user
    // name, and sends them to all broadcast subscribers.
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            // Add username before message.
            let channel_message = ChannelMessage {
                user_name: name.to_owned(),
                message_type: MessageType::Message,
                message: text,
            };
            let message = serde_json::to_string(&channel_message).unwrap();
            let _ = tx.send(message);
        }
    });

    // If any one of the tasks run to completion, we abort the other.
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    };

    // Send "user left" message (similar to "joined" above).
    let channel_message = ChannelMessage {
        user_name: channel_user.user_name.to_owned(),
        message_type: MessageType::Left,
        message: format!("{} left.", channel_user.user_name),
    };
    let msg = serde_json::to_string(&channel_message).unwrap();
    tracing::debug!("{msg}");
    let _ = state_tx.send(msg);
}

fn check_channel(state: &AppState, channel_name: &String) -> Sender<String> {
    let mut channel_list = state.channel_list.lock().unwrap();

    if !channel_list.contains(&channel_name) {
        channel_list.push(channel_name.to_owned());
        let mut channel_tx_list = state.channel_tx_list.lock().unwrap();
        let (tx, _rx) = broadcast::channel(100);
        channel_tx_list.push(tx);
    }

    let channel_index = channel_list
        .iter()
        .enumerate()
        .find(|(_, name)| *name == channel_name)
        .unwrap()
        .0;
    let mut channel_tx_list = state.channel_tx_list.lock().unwrap();
    channel_tx_list[channel_index].clone()
}

// Include utf-8 file at **compile** time.
async fn index() -> Html<&'static str> {
    Html(std::include_str!("../static/index.html"))
}
