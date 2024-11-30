use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite::protocol::Message, WebSocketStream, MaybeTlsStream};
use serde_json::json;
use uuid::Uuid;
use futures_util::{SinkExt, StreamExt};
use http::Response;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use futures_util::stream::{SplitStream, SplitSink};
use std::env;
use url::Url;

const WS_URL: &str = "wss://dashscope.aliyuncs.com/api-ws/v1/inference/";
const OUTPUT_FILE: &str = "output.mp3";

#[tokio::main]
async fn main() {
    let api_key = match env::var("DASHSCOPE_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Error: DASHSCOPE_API_KEY environment variable not set");
            return;
        }
    };
    
    println!("Clearing output file...");
    if let Err(e) = clear_output_file(OUTPUT_FILE).await {
        eprintln!("Failed to clear output file: {}", e);
        return;
    }

    println!("Connecting to WebSocket server...");
    match connect_websocket(&api_key).await {
        Ok((stream, _)) => {
            println!("Successfully connected to WebSocket server");
            let (write, read) = stream.split();
            let task_started = Arc::new(AtomicBool::new(false));
            let task_started_clone = task_started.clone();

            let result_receiver_task = tokio::spawn(async move {
                start_result_receiver(read, task_started_clone).await;
            });

            let mut write_half = write;
            println!("Sending run-task command...");
            let task_id = match send_run_task_cmd(&mut write_half).await {
                Ok(id) => {
                    println!("Run-task command sent successfully, task_id: {}", id);
                    id
                },
                Err(e) => {
                    eprintln!("Failed to send run-task command: {}", e);
                    return;
                }
            };

            println!("Waiting for task to start...");
            while !task_started.load(Ordering::SeqCst) {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            println!("Sending continue-task command...");
            if let Err(e) = send_continue_task_cmd(&mut write_half, &task_id).await {
                eprintln!("Failed to send continue-task command: {}", e);
                return;
            }

            println!("Sending finish-task command...");
            if let Err(e) = send_finish_task_cmd(&mut write_half, &task_id).await {
                eprintln!("Failed to send finish-task command: {}", e);
                return;
            }

            println!("Waiting for result receiver task to complete...");
            result_receiver_task.await.unwrap();
            println!("Task completed successfully!");
        },
        Err(e) => {
            eprintln!("Failed to connect to WebSocket: {}", e);
            eprintln!("Please check your API key and internet connection");
        }
    }
}

async fn connect_websocket(api_key: &str) -> Result<(WebSocketStream<MaybeTlsStream<TcpStream>>, Response<()>), Box<dyn std::error::Error>> {
    use tokio_tungstenite::tungstenite::handshake::client::Request;
    use tokio_tungstenite::tungstenite::http::header::{HeaderMap, HeaderValue};
    
    // 构建请求
    let request = Request::builder()
        .uri(format!("{}?token={}", WS_URL, api_key))
        .header("X-DashScope-DataInspection", "enable")
        .header("Authorization", format!("bearer {}", api_key))
        .body(())?;

    // 连接 WebSocket
    let (ws_stream, response) = tokio_tungstenite::connect_async(request).await?;

    Ok((ws_stream, response))
}

async fn send_run_task_cmd(
    stream: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>
) -> Result<String, Box<dyn std::error::Error>> {
    let task_id = Uuid::new_v4().to_string();
    let cmd = json!({
        "header": {
            "action": "run-task",
            "task_id": task_id,
            "streaming": "duplex"
        },
        "payload": {
            "task_group": "audio",
            "task": "tts",
            "function": "SpeechSynthesizer",
            "model": "cosyvoice-v1",
            "parameters": {
                "text_type": "PlainText",
                "voice": "longxiaochun",
                "format": "mp3",
                "sample_rate": 22050,
                "volume": 50,
                "rate": 1,
                "pitch": 1
            },
            "input": {}
        }
    });
    stream.send(Message::Text(cmd.to_string())).await?;
    Ok(task_id)
}

async fn send_continue_task_cmd(
    stream: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    task_id: &str
) -> Result<(), Box<dyn std::error::Error>> {
    let texts = vec![
        "床前明月光", 
        "疑是地上霜", 
        "举头望明月", 
        "低头思故乡"
    ];

    for text in texts {
        let cmd = json!({
            "header": {
                "action": "continue-task",
                "task_id": task_id,
                "streaming": "duplex"
            },
            "payload": {
                "input": {
                    "text": text
                }
            }
        });
        stream.send(Message::Text(cmd.to_string())).await?;
    }

    Ok(())
}

async fn send_finish_task_cmd(
    stream: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    task_id: &str
) -> Result<(), Box<dyn std::error::Error>> {
    let cmd = json!({
        "header": {
            "action": "finish-task",
            "task_id": task_id,
            "streaming": "duplex"
        },
        "payload": {
            "input": {}
        }
    });
    stream.send(Message::Text(cmd.to_string())).await?;
    Ok(())
}

async fn start_result_receiver(
    mut stream: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    task_started: Arc<AtomicBool>
) {
    while let Some(msg) = stream.next().await {
        match msg {
            Ok(message) => match message {
                Message::Binary(data) => {
                    if let Err(e) = write_binary_data_to_file(&data, OUTPUT_FILE).await {
                        eprintln!("Failed to write binary data to file: {}", e);
                        break;
                    }
                },
                Message::Text(text) => {
                    if let Err(e) = handle_event(text, &task_started) {
                        eprintln!("Failed to handle event: {}", e);
                    }
                },
                _ => (),
            },
            Err(e) => {
                eprintln!("Received error from WebSocket: {}", e);
                break;
            }
        }
    }
}

fn handle_event(event_str: String, task_started: &Arc<AtomicBool>) -> Result<(), Box<dyn std::error::Error>> {
    let event: serde_json::Value = serde_json::from_str(&event_str)?;
    println!("Received event: {}", event_str);
    match event["header"]["event"].as_str() {
        Some("task-started") => {
            println!("Task started successfully");
            task_started.store(true, Ordering::SeqCst);
        },
        Some("result-generated") => {
            println!("Result generated");
        },
        Some("task-finished") => {
            println!("Task finished successfully");
            return Ok(());
        },
        Some("task-failed") => {
            let error_message = event["header"]["error_message"]
                .as_str()
                .unwrap_or("Unknown error");
            eprintln!("Task failed: {}", error_message);
            return Ok(());
        },
        _ => println!("Received unexpected event: {:?}", event),
    }
    Ok(())
}

async fn write_binary_data_to_file(data: &[u8], file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open(file_path).await?;

    file.write_all(data).await?;
    Ok(())
}

async fn clear_output_file(file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = OpenOptions::new()
        .truncate(true)
        .create(true)
        .write(true)
        .open(file_path).await?;

    file.set_len(0).await?;
    Ok(())
}



