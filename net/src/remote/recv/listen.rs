use std::sync::Arc;

use eyre::{eyre, Context};
use futures::StreamExt;
use log::error;
use quinn::{Connecting, Incoming, NewConnection};
use tokio::spawn;

use crate::{send, AuthToken, Connection, Service};

pub async fn listen<S: Service>(mut incoming: Incoming, service: Arc<S>) {
    while let Some(connecting) = incoming.next().await {
        let service = service.clone();
        spawn(async move {
            if let Err(error) = accept(connecting, service).await {
                error!("{:?}", error)
            }
        });
    }
}

async fn accept<S: Service>(connecting: Connecting, service: Arc<S>) -> eyre::Result<()> {
    let NewConnection {
        connection,
        bi_streams,
        mut uni_streams,
        datagrams,
        ..
    } = connecting
        .await
        .wrap_err("could not establish new connection")?;
    let recv = uni_streams
        .next()
        .await
        .ok_or_else(|| eyre!("no token was received"))?
        .wrap_err("could not open token stream")?;
    let buffer = recv
        .read_to_end(service.token_size_limit())
        .await
        .wrap_err("could not read token")?;
    let token = std::str::from_utf8(&buffer).wrap_err("token must be utf-8 encoded")?;
    let connection = Connection::remote(send::Connection::new(connection));
    let receiver = service
        .authenticate(connection, AuthToken::Remote(token))
        .wrap_err("token authentication failed")?;
    super::connection(bi_streams, uni_streams, datagrams, receiver).await;
    Ok(())
}
