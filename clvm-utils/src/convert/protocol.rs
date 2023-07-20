use std::io::Cursor;

use chia_protocol::{Program, Streamable};
use clvmr::{
    allocator::{NodePtr, SExp},
    serde::{node_from_bytes, node_to_bytes},
    Allocator,
};

use crate::{Error, FromClvm, Result, ToClvm};

impl FromClvm for Program {
    fn from_clvm(a: &Allocator, node: NodePtr) -> Result<Self> {
        if let SExp::Atom() = a.sexp(node) {
            let bytes = node_to_bytes(a, node).map_err(|error| Error::Reason(error.to_string()))?;
            Self::parse(&mut Cursor::new(&bytes)).map_err(|error| Error::Reason(error.to_string()))
        } else {
            Err(Error::ExpectedAtom(node))
        }
    }
}

impl ToClvm for Program {
    fn to_clvm(&self, a: &mut Allocator) -> Result<NodePtr> {
        node_from_bytes(a, self.as_ref()).map_err(|error| Error::Reason(error.to_string()))
    }
}