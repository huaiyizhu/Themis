pub mod client;
pub mod server;

pub mod proto {
    tonic::include_proto!("themis.v1");
}

pub use proto::themis_service_client::ThemisServiceClient;
pub use proto::themis_service_server::{ThemisService, ThemisServiceServer};
pub use proto::*;

pub const DEFAULT_GRPC_ADDR: &str = "127.0.0.1:50051";

pub fn grpc_addr(port: u16) -> String {
    format!("127.0.0.1:{port}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_addr_uses_port() {
        assert_eq!(grpc_addr(50051), "127.0.0.1:50051");
    }
}
