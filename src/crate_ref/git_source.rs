use crate::{crate_ref::CrateRef, errors::CarguixError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitSource {}

impl CrateRef for GitSource {
    fn crate_name(&self) -> String {
        unimplemented!()
    }

    fn package_name(&self) -> String {
        unimplemented!()
    }

    fn version(&self) -> String {
        unimplemented!()
    }

    fn source(&self) -> String {
        unimplemented!()
    }

    fn dependencies(&self) -> Result<Vec<Box<dyn CrateRef>>, CarguixError> {
        unimplemented!()
    }
}
