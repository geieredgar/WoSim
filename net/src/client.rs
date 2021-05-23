pub enum Client<'a> {
    Local,
    Remote { token: &'a str },
}
