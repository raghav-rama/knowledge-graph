use crate::AppState;
use crate::routes::types::GraphSearchResponse;
use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use std::sync::Arc;
use x402_axum::{IntoPriceTag, X402Middleware};
use x402_rs::address_evm;
use x402_rs::network::{Network, USDCDeployment};
use x402_rs::telemetry::Telemetry;
use x402_rs::types::{EvmAddress, MixedAddress};

pub fn x402_route() -> Router<Arc<AppState>> {
    let x402 = X402Middleware::try_from("https://facilitator.x402.rs").unwrap();
    let address = address_evm!("0x2C1b291B3946e06ED41FB543B18a21558eBa3d62");
    let usdc = USDCDeployment::by_network(Network::BaseSepolia).pay_to(address);
    Router::new().route(
        "/search-graph",
        get(handler).layer(
            x402.with_description("Search for a term on the knowledge graph")
                .with_price_tag(usdc.amount(0.01).unwrap()),
        ),
    )
}

async fn handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<GraphSearchResponse>, (StatusCode, String)> {
    Ok(Json(GraphSearchResponse {
        message: "Hello from x402".to_string(),
    }))
}
