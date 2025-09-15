use axum::{
    extract::Json,
    http::StatusCode,
    response::{IntoResponse, Response, Sse},
    routing::post,
    Router,
};
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use chrono::{Duration, Utc};
use futures_util::stream::{self, Stream, StreamExt};
use jsonwebtoken::{decode, DecodingKey, Validation, encode, Header, EncodingKey};
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, time::Duration as StdDuration};
use tokio::time::sleep;
use uuid::Uuid;

// --- Secret Key for JWT ---
const JWT_SECRET: &str = "your-super-secret-and-long-key";

// --- Data Structures ---

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String, // Subject (e.g., user_id)
    company: String,
    exp: usize, // Expiration time (timestamp)
}

#[derive(Deserialize, Debug)]
struct ChatRequest {
    messages: Vec<Message>,
    model: String,
    stream: Option<bool>,
    temperature: Option<f64>,
    max_tokens: Option<u32>,
}

#[derive(Deserialize, Debug)]
struct Message {
    role: String,
    content: String,
}

// ... other response structs are the same ...

#[derive(Serialize)]
struct ChatCompletion { id: String, object: String, created: u64, model: String, choices: Vec<Choice> }
#[derive(Serialize)]
struct Choice { index: u32, message: ChatMessage, finish_reason: String }
#[derive(Serialize)]
struct ChatMessage { role: String, content: String }
#[derive(Serialize)]
struct ChatCompletionChunk { id: String, object: String, created: u64, model: String, choices: Vec<ChunkChoice> }
#[derive(Serialize)]
struct ChunkChoice { index: u32, delta: Delta, finish_reason: Option<String> }
#[derive(Serialize, Default)]
struct Delta { #[serde(skip_serializing_if = "Option::is_none")] role: Option<String>, #[serde(skip_serializing_if = "Option::is_none")] content: Option<String> }


// --- Main Application ---

#[tokio::main]
async fn main() {
    // A separate router for a helper endpoint to generate a token for testing
    let app = Router::new()
        .route("/v1/chat/completions", post(chat_completions_handler))
        .route("/generate-token", post(generate_token_handler));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("🚀 Mock LLM Server listening on http://127.0.0.1:3000");
    println!("🔑 Send a POST request to /generate-token to get a test JWT.");

    axum::serve(listener, app).await.unwrap();
}

// --- Route Handlers ---

async fn chat_completions_handler(
    auth_header: Option<TypedHeader<Authorization<Bearer>>>,
    Json(payload): Json<ChatRequest>,
) -> Result<Response, StatusCode> {
    let claims = match auth_header {
        Some(TypedHeader(Authorization(bearer))) => {
            match decode::<Claims>(
                bearer.token(),
                &DecodingKey::from_secret(JWT_SECRET.as_ref()),
                &Validation::default(),
            ) {
                Ok(token_data) => token_data.claims,
                Err(_) => return Err(StatusCode::UNAUTHORIZED),
            }
        }
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    println!("✅ Token validated successfully!");
    println!("👤 Subject: {}, Company: {}", claims.sub, claims.company);

    let model = payload.model.clone();
    if payload.stream.unwrap_or(false) {
        Ok(sse_handler(model).await.into_response())
    } else {
        Ok(json_handler(model).await.into_response())
    }
}

#[derive(Deserialize)]
struct TokenGenerationRequest {
    user_id: String,
    company: String,
}

async fn generate_token_handler(Json(req): Json<TokenGenerationRequest>) -> impl IntoResponse {
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(1))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        sub: req.user_id,
        company: req.company,
        exp: expiration as usize,
    };

    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(JWT_SECRET.as_ref()))
        .unwrap();

    Json(serde_json::json!({ "token": token }))
}

// ... json_handler and sse_handler are the same as before ...
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
                    let chunk = ChatCompletionChunk { id: format!("cmpl-{}", Uuid::new_v4()), object: "chat.completion.chunk".to_string(), created: std::time::UNIX_EPOCH.elapsed().unwrap().as_secs(), model: model_name, choices: vec![ChunkChoice { index: 0, delta: Delta { role: None, content: Some(word.to_string()) }, finish_reason: None }] };
                    let event = axum::response::sse::Event::default().data(serde_json::to_string(&chunk).unwrap());
                    sleep(StdDuration::from_millis(50)).await;
                    Some((Ok(event), words))
                }
                None => None,
            }
        }
    }).chain(stream::iter(vec![
        Ok(axum::response::sse::Event::default().data(serde_json::to_string(&ChatCompletionChunk { id: format!("cmpl-{}", Uuid::new_v4()), object: "chat.completion.chunk".to_string(), created: std::time::UNIX_EPOCH.elapsed().unwrap().as_secs(), model, choices: vec![ChunkChoice { index: 0, delta: Delta::default(), finish_reason: Some("stop".to_string()) }] }).unwrap())),
        Ok(axum::response::sse::Event::default().data("[DONE]"))
    ]));

    Sse::new(stream)
}