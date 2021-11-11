pub trait Object {
    fn get_data(&self) -> Vec<u8>;

    fn type_(&self) -> &str;

    // fn set_oid(&mut self, oid: String);

    fn get_oid(&self) -> &str;
}
