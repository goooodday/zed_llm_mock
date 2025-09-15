# Zed LLM Mock Server

A simple, OpenAI-compatible mock server written in Rust. This server is designed to test the custom LLM provider feature in the Zed code editor and other API clients.

This server uses JWT (JSON Web Token) for authentication.

## Features

- Emulates the `/v1/chat/completions` endpoint with JWT authentication.
- Provides a `/generate-token` endpoint to get a test token.
- Supports both streaming (Server-Sent Events) and non-streaming JSON responses.
- Lightweight and easy to run locally.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) toolchain

## Installation & Running

1. Clone this repository.
2. Navigate to the project directory:
   ```bash
   cd zed_llm_mock
   ```
3. Run the server:
   ```bash
   cargo run
   ```
The server will start and listen on `http://127.0.0.1:3000`.

## Authentication

All requests to `/v1/chat/completions` require a valid JWT in the `Authorization` header.

### 1. Generate a Test Token

Use the following command to get a 1-hour valid test token. You can change the `user_id` and `company` values.

```bash
curl -X POST http://127.0.0.1:3000/generate-token \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "test-user-123",
    "company": "MyCompany"
  }'
```

This will return a JSON object with the token. Copy the token value.

### 2. Make an Authenticated Request

Use the copied token to make requests to the chat completions endpoint.

```bash
# Replace <YOUR_COPIED_TOKEN> with the token you received
TOKEN="<YOUR_COPIED_TOKEN>"

curl http://127.0.0.1:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "model": "mock-model",
    "messages": [{"role": "user", "content": "Hello with JWT"}]
  }'
```

## Zed Configuration Example

Below is an example of how to configure Zed to point to this mock server. 

```json
{
  "agent": {
    "default_model": {
      "provider": "rust_mock",
      "model": "mock-model"
    }
  },
  "language_models": {
    "openai_compatible": {
      "rust_mock": {
        "api_url": "http://127.0.0.1:3000/v1",
        "available_models": [
          {
            "name": "mock-model",
            "display_name": "Rust Mock",
            "max_tokens": 8000,
            "max_output_tokens": 4096
          }
        ]
      }
    }
  }
}
```

**🚨 Important Warning:** Zed's current UI for OpenAI-compatible providers does not have a standard field to inject a Bearer Token for authentication. Because of this, any request from the Zed assistant to this server will **fail with a `401 Unauthorized` error**. The configuration above will make the model appear in Zed, but it will not be usable due to the server's JWT authentication.
