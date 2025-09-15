# Zed LLM Mock Server

A simple, OpenAI-compatible mock server written in Rust. This server is designed to test the custom LLM provider feature in the Zed code editor.

## Features

- Emulates the `/v1/chat/completions` endpoint.
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

## Zed Configuration

To use this mock server in Zed, open your `settings.json` (`cmd + ,`) and add the following configuration under `language_models.openai_compatible`:

```json
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
```

After saving the settings, you can select "Rust Mock" from the model list in the Zed assistant.
