use std::sync::Arc;

use actor::Address;

use futures::StreamExt;

use quinn::{Connecting, Incoming, NewConnection, VarInt};
use tokio::spawn;

use crate::{session, Authenticator, EstablishConnectionError, Reader, RemoteSender};

pub fn listen<A: Authenticator>(mut incoming: Incoming, authenticator: Arc<A>) {
    spawn(async move {
        while let Some(connecting) = incoming.next().await {
            spawn(accept(connecting, authenticator.clone()));
        }
    });
}

async fn accept<A: Authenticator>(
    connecting: Connecting,
    authenticator: Arc<A>,
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
    let token = Reader::recv(recv, A::token_size_limit())
        .await?
        .read()
        .map_err(EstablishConnectionError::Deserialize)?;
    let sender = RemoteSender(connection.clone());
    let client = Address::new(move |message| {
        sender.send(message);
        Ok(())
    });
    let receiver = match authenticator.authenticate(client, token) {
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
