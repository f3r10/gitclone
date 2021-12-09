use anyhow::Result;

pub trait Object {
    fn get_data(&self) -> Result<Vec<u8>>;

    fn type_(&self) -> &str;

    fn get_oid(&self) -> Result<Vec<u8>>;
}
