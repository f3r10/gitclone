use anyhow::Result;

pub trait Object {
    fn get_data(&self) -> Result<Vec<u8>>;

    fn type_(&self) -> &str;

    // fn set_oid(&mut self, oid: String);

    fn get_oid(&mut self) -> Result<Vec<u8>>;
}
