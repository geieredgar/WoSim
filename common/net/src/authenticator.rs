pub trait Authenticator: Send + Sync {
    fn login(&self, token: Vec<u8>) -> Result<u64, String>;
    fn logout(&self, id: u64);
}
