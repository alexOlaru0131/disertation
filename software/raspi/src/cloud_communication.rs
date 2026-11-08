use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

type ClientSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct CloudCommunicator {
    pub socket: ClientSocket,
}

impl CloudCommunicator {
    pub fn new(socket: ClientSocket) -> Self {
        Self {
            socket,
        }
    }

    pub async fn close(&mut self) -> Result<(), tokio_tungstenite::tungstenite::Error> {
        self.socket.close(None).await
    }
}
