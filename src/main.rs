use tonic::transport::Server;

use battleship::battleship_server::BattleshipServer;
use services::battleship::BattleShipService;

pub mod battleship {
    tonic::include_proto!("battleship");
}

pub mod services {
    pub mod battleship;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let service = BattleShipService::default();
    let grpc_service = BattleshipServer::new(service);

    Server::builder().add_service(service).serve(addr).await?;

    Ok(())
}
