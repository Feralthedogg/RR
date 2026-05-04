use super::*;

#[derive(Clone, Debug)]
pub(crate) enum User {
    Block(BlockId),
    Value(ValueId),
}

#[cfg(test)]
#[path = "../sccp/tests.rs"]
pub(crate) mod tests;
