use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ProtocolError {
    #[error("Command `{0:#X}` is not exist's")]
    UnknownCommand(u8),

    #[error("Version `{0:#X} is not exist's or very old")]
    UnknownVersion(u8),

    #[error("Pocket is incorrect! Invalid magic counter {0:#X}")]
    MagicIncorrect(u8),

    #[error("Incorrect pocket header. Incompled")]
    IncompledHeader,
}
