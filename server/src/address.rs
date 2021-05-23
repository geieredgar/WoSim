pub enum ServerAddress {
    Local,
    Remote { address: String, token: String },
}
