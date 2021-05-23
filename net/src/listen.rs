use std::sync::Arc;

use actor::Address;

use futures::StreamExt;

use quinn::{Connecting, Incoming, NewConnection, VarInt};
use tokio::spawn;

use crate::{session, Client, EstablishConnectionError, RemoteSender, Server};

pub fn listen<S: Server>(mut incoming: Incoming, server: Arc<S>) {
    spawn(async move {
        while let Some(connecting) = incoming.next().await {
            spawn(accept(connecting, server.clone()));
        }
    });
}

async fn accept<S: Server>(
    connecting: Connecting,
    server: Arc<S>,
) -> Result<(), EstablishConnectionError> {
    let NewConnection {
        connection,
        bi_streams,
        mut uni_streams,
        datagrams,
        ..
    } = connecting.await?;
    let recv = uni_streams
        .next()
        .await
        .ok_or(EstablishConnectionError::TokenMissing)??;
    let buffer = recv.read_to_end(S::token_size_limit()).await?;
    let token = std::str::from_utf8(&buffer).map_err(EstablishConnectionError::InvalidToken)?;
    let sender = RemoteSender(connection.clone());
    let address = Address::new(move |message| {
        sender.send(message);
        Ok(())
    });
    let receiver = match server.authenticate(Client::Remote { token }, address) {
        Ok(receiver) => receiver,
        Err(error) => {
            let reason = error.to_string();
            connection.close(VarInt::from_u32(1), reason.as_bytes());
            return Err(EstablishConnectionError::TokenRejected(reason));
        }
    };
    session(bi_streams, uni_streams, datagrams, receiver).await;
    Ok(())
}
