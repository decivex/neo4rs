use crate::connection::Connection;
use crate::errors::Error;
use crate::messages::*;
use crate::query::*;
use crate::version::Version;
use async_trait::async_trait;
use bb8::ManageConnection;
use bb8::PooledConnection;
use core::ops::Deref;
use core::ops::DerefMut;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct GraphConnection {
    connection: Arc<Mutex<Connection>>,
}

impl GraphConnection {
    pub fn query(&self, q: &str) -> Query {
        Query::new(q.to_owned(), self.connection.clone())
    }
}

pub struct GraphConnectionManager {
    uri: String,
    user: String,
    password: String,
}

impl GraphConnectionManager {
    pub fn new(uri: &str, user: &str, password: &str) -> GraphConnectionManager {
        GraphConnectionManager {
            uri: uri.to_owned(),
            user: user.to_owned(),
            password: password.to_owned(),
        }
    }
}

#[async_trait]
impl ManageConnection for GraphConnectionManager {
    type Connection = GraphConnection;
    type Error = Error;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        let (mut connection, version) = Connection::new(self.uri.clone()).await?;
        let hello = BoltRequest::hello("neo4rs", self.user.clone(), self.password.clone());
        match connection.send_recv(hello).await? {
            BoltResponse::SuccessMessage(msg) => Ok(GraphConnection {
                connection: Arc::new(Mutex::new(connection)),
            }),
            BoltResponse::FailureMessage(msg) => Err(Error::AuthenticationError {
                detail: msg.get("message").unwrap(),
            }),
            _ => Err(Error::UnexpectedMessage),
        }
    }

    async fn is_valid(&self, conn: &mut PooledConnection<'_, Self>) -> Result<(), Self::Error> {
        Ok(())
    }

    fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
        false
    }
}

pub struct ConnectionManager {
    uri: String,
    user: String,
    password: String,
}

impl ConnectionManager {
    pub fn new(uri: &str, user: &str, password: &str) -> ConnectionManager {
        ConnectionManager {
            uri: uri.to_owned(),
            user: user.to_owned(),
            password: password.to_owned(),
        }
    }
}

pub struct ConnectionWrapper {
    pub version: Version,
    inner: Connection,
}

impl Deref for ConnectionWrapper {
    type Target = Connection;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for ConnectionWrapper {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[async_trait]
impl ManageConnection for ConnectionManager {
    type Connection = ConnectionWrapper;
    type Error = Error;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        let (mut inner, version) = Connection::new(self.uri.clone()).await?;
        let hello = BoltRequest::hello("neo4rs", self.user.clone(), self.password.clone());
        match inner.send_recv(hello).await? {
            BoltResponse::SuccessMessage(msg) => Ok(ConnectionWrapper { version, inner }),
            BoltResponse::FailureMessage(msg) => Err(Error::AuthenticationError {
                detail: msg.get("message").unwrap(),
            }),
            _ => Err(Error::UnexpectedMessage),
        }
    }

    async fn is_valid(&self, conn: &mut PooledConnection<'_, Self>) -> Result<(), Self::Error> {
        Ok(())
    }

    fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
        false
    }
}