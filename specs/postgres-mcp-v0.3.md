# Postgres MCP v0.3

In this version, we shall support both stdio mode and SSE mode. When `postgres-mcp` is started with `stdio` subcommand, it will be in stdio mode. When it is started with `sse` subcommand, it will be in SSE mode. Use clap to support both subcommand. For sse subcommand it should allow an extra `--port` or `-p` for SSE server to bind.

## Implementations

### New dependencies

Please use the version specified here.

- [axum](https://github.com/tokio-rs/axum): 0.8.0, enable macros feature
- [tokio-stream](https://github.com/tokio-rs/tokio/tree/master/tokio-stream): 0.1.0
- [tokio-util](https://github.com/tokio-rs/tokio/tree/master/tokio-util): 0.7.0, enable codec feature

### New features

#### Parse subcommand and args using clap

Below is an example of how to use subcommand and args:

```rust
use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Optional name to operate on
    name: Option<String>,

    /// Sets a custom config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// does testing things
    Test {
        /// lists test values
        #[arg(short, long)]
        list: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    // You can check the value provided by positional arguments, or option arguments
    if let Some(name) = cli.name.as_deref() {
        println!("Value for name: {name}");
    }

    if let Some(config_path) = cli.config.as_deref() {
        println!("Value for config: {}", config_path.display());
    }

    // You can see how many times a particular flag or argument occurred
    // Note, only flags can have multiple occurrences
    match cli.debug {
        0 => println!("Debug mode is off"),
        1 => println!("Debug mode is kind of on"),
        2 => println!("Debug mode is on"),
        _ => println!("Don't be crazy"),
    }

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Some(Commands::Test { list }) => {
            if *list {
                println!("Printing testing lists...");
            } else {
                println!("Not printing testing lists...");
            }
        }
        None => {}
    }

    // Continued program logic goes here...
}

```

#### SSE support

`postgres-mcp sse` will use SSE to send events to the client. You need axum to support SSE. Below is an example:

```rust
use rmcp::transport::sse_server::{SseServer, SseServerConfig};
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    {self},
};
mod common;
use common::counter::Counter;

async fn sse(port: u16) -> anyhow::Result<()> {
    let addr = format!("0.0.0.0:{}", port);
    let config = SseServerConfig {
        bind: addr.parse()?,
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: tokio_util::sync::CancellationToken::new(),
        sse_keep_alive: None,
    };

    let (sse_server, router) = SseServer::new(config);

    // Do something with the router, e.g., add routes or middleware

    let listener = tokio::net::TcpListener::bind(sse_server.config.bind).await?;

    let ct = sse_server.config.ct.child_token();

    let server = axum::serve(listener, router).with_graceful_shutdown(async move {
        ct.cancelled().await;
        tracing::info!("sse server cancelled");
    });

    tokio::spawn(async move {
        if let Err(e) = server.await {
            tracing::error!(error = %e, "sse server shutdown with error");
        }
    });

    let ct = sse_server.with_service(|| Counter::new());

    tokio::signal::ctrl_c().await?;
    ct.cancel();
    Ok(())
}
```
