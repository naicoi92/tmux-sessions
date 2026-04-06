pub struct Session {
    pub name: String,
}

impl Session {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_session() {
        let session = Session::new("test");
        assert_eq!(session.name, "test");
    }
}
