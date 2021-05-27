pub enum AuthToken<'a> {
    Local(&'a str),
    Remote(&'a str),
}
