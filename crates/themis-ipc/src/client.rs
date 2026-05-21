use crate::proto::themis_service_client::ThemisServiceClient;
use crate::grpc_addr;
use tonic::transport::Channel;

pub async fn connect(port: u16) -> anyhow::Result<ThemisServiceClient<Channel>> {
    let addr = grpc_addr(port);
    let client = ThemisServiceClient::connect(format!("http://{addr}")).await?;
    Ok(client)
}
