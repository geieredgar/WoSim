use std::sync::Arc;

use actor::Address;

use futures::StreamExt;

use quinn::{Connecting, Incoming, NewConnection, VarInt};
use tokio::spawn;

use crate::{
    session, Authenticator, EstablishConnectionError, Message, Reader, RemoteSender, SessionMessage,
};

pub fn listen<M: Message, A: Authenticator>(
    mut incoming: Incoming,
    authenticator: Arc<A>,
    receiver: Address<SessionMessage<A::Identity, M>>,
) {
    spawn(async move {
        while let Some(connecting) = incoming.next().await {
            spawn(accept(connecting, authenticator.clone(), receiver.clone()));
        }
    });
}

async fn accept<M: Message, A: Authenticator>(
    connecting: Connecting,
    authenticator: Arc<A>,
    receiver: Address<SessionMessage<A::Identity, M>>,
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
    let identity = match authenticator.authenticate(client, token) {
        Ok(identity) => identity,
        Err(error) => {
            let reason = error.to_string();
            connection.close(VarInt::from_u32(1), reason.as_bytes());
            return Err(EstablishConnectionError::TokenRejected(reason));
        }
    };
    session(bi_streams, uni_streams, datagrams, receiver, identity).await;
    Ok(())
}
