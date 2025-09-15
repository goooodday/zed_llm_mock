use axum::{
    extract::Json,
    http::StatusCode,
    response::{IntoResponse, Response, Sse},
    routing::post,
    Router,
};
use futures_util::stream::{self, Stream};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, time::Duration};
use tokio::time::sleep;
use uuid::Uuid;

#[derive(Deserialize, Debug)]
struct ChatRequest {
    messages: Vec<Message>,
    model: String,
    stream: Option<bool>,
}

#[derive(Deserialize, Debug)]
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatCompletion {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<Choice>,
}

#[derive(Serialize)]
struct Choice {
    index: u32,
    message: ChatMessage,
    finish_reason: String,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatCompletionChunk {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<ChunkChoice>,
}

#[derive(Serialize)]
struct ChunkChoice {
    index: u32,
    delta: Delta,
    finish_reason: Option<String>,
}

#[derive(Serialize, Default)]
struct Delta {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/v1/chat/completions", post(chat_completions_handler));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("🚀 Mock LLM Server listening on http://127.0.0.1:3000");

    axum::serve(listener, app).await.unwrap();
}

async fn chat_completions_handler(
    Json(payload): Json<ChatRequest>,
) -> Result<Response, StatusCode> {
    let model = payload.model.clone();
    println!("Received request for model: {}", model);
    println!("Streaming enabled: {:?}", payload.stream);
    println!("Messages: {:#?}", payload.messages);


    if payload.stream.unwrap_or(false) {
        Ok(sse_handler(model).await.into_response())
    } else {
        Ok(json_handler(model).await.into_response())
    }
}

async fn json_handler(model: String) -> Json<ChatCompletion> {
    Json(ChatCompletion {
        id: format!("cmpl-{}", Uuid::new_v4()),
        object: "chat.completion".to_string(),
        created: std::time::UNIX_EPOCH.elapsed().unwrap().as_secs(),
        model,
        choices: vec![Choice {
            index: 0,
            message: ChatMessage {
                role: "assistant".to_string(),
                content: "Hello from your local Rust mock server! This is a non-streaming response.".to_string(),
            },
            finish_reason: "stop".to_string(),
        }],
    })
}

async fn sse_handler(model: String) -> Sse<impl Stream<Item = Result<axum::response::sse::Event, Infallible>>> {
    let response_words = vec![
        "Hello ", "from ", "your ", "local ", "Rust ", "mock ", "server! ",
        "This ", "is ", "a ", "streaming ", "response.",
    ];

    let model_clone = model.clone();
    let stream = stream::unfold(response_words.into_iter(), move |mut words| {
        let model_name = model_clone.clone();
        async move {
            match words.next() {
                Some(word) => {
                    let chunk = ChatCompletionChunk {
                        id: format!("cmpl-{}", Uuid::new_v4()),
                        object: "chat.completion.chunk".to_string(),
                        created: std::time::UNIX_EPOCH.elapsed().unwrap().as_secs(),
                        model: model_name,
                        choices: vec![ChunkChoice {
                            index: 0,
                            delta: Delta {
                                role: None,
                                content: Some(word.to_string()),
                            },
                            finish_reason: None,
                        }],
                    };
                    let event = axum::response::sse::Event::default()
                        .data(serde_json::to_string(&chunk).unwrap());
                    
                    sleep(Duration::from_millis(50)).await;
                    Some((Ok(event), words))
                }
                None => {
                    // The unfold stream is finished, the chained iterator will provide the final events.
                    None
                }
            }
        }
    }).chain(stream::iter(vec![
        Ok(axum::response::sse::Event::default().data(
            serde_json::to_string(&ChatCompletionChunk {
                id: format!("cmpl-{}", Uuid::new_v4()),
                object: "chat.completion.chunk".to_string(),
                created: std::time::UNIX_EPOCH.elapsed().unwrap().as_secs(),
                model,
                choices: vec![ChunkChoice {
                    index: 0,
                    delta: Delta::default(),
                    finish_reason: Some("stop".to_string()),
                }],
            }).unwrap()
        )),
        Ok(axum::response::sse::Event::default().data("[DONE]"))
    ]));

    Sse::new(stream)
}
