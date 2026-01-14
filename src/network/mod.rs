use crate::pbft::PBFTMessage;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde_json::json;
use std::sync::Arc;

pub struct NetworkHandler {
    pub on_message: Arc<dyn Fn(PBFTMessage) -> bool + Send + Sync>,
}

impl NetworkHandler {
    pub fn new<F>(handler: F) -> Self 
    where
        F: Fn(PBFTMessage) -> bool + Send + Sync + 'static,
    {
        NetworkHandler {
            on_message: Arc::new(handler),
        }
    }
}

async fn receive_message(
    msg: web::Json<PBFTMessage>,
    handler: web::Data<Arc<NetworkHandler>>,
) -> impl Responder {
    let result = (handler.on_message)(msg.into_inner());
    HttpResponse::Ok().json(json!({
        "status": if result { "accepted" } else { "pending" },
        "quorum_reached": result
    }))
}

async fn health() -> impl Responder {
    HttpResponse::Ok().json(json!({"status": "healthy"}))
}

pub async fn start_server(
    port: u16,
    handler: Arc<NetworkHandler>,
) -> std::io::Result<()> {
    let handler_data = web::Data::new(handler);
    
    println!("[Network] Starting HTTP server on port {}", port);
    
    HttpServer::new(move || {
        App::new()
            .app_data(handler_data.clone())
            .route("/message", web::post().to(receive_message))
            .route("/health", web::get().to(health))
    })
    .bind(("127.0.0.1", port))?
    .run()
    .await
}

pub async fn send_message(url: &str, message: &PBFTMessage) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let response = client
        .post(&format!("http://{}/message", url))
        .json(message)
        .send()
        .await?;
    
    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("HTTP error: {}", response.status()).into())
    }
}

pub async fn broadcast_message(
    message: &PBFTMessage,
    node_addresses: &[String],
    current_node_port: u16,
) {
    for addr in node_addresses {
        if let Some(port_str) = addr.split(':').last() {
            if let Ok(port) = port_str.parse::<u16>() {
                if port == current_node_port {
                    continue;
                }
            }
        }
        
        if let Err(e) = send_message(addr, message).await {
            eprintln!("[Warning] [Network] Failed to send to {}: {}", addr, e);
        }
    }
}
